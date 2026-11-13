param([string]$Executable = 'src-tauri/target/release/Bimu.exe')
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class BimuNativeWindow {
  public delegate bool EnumProc(IntPtr hwnd, IntPtr arg);
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct Point { public int X,Y; }
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback, IntPtr arg);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd,out Rect value);
  [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hwnd,out Rect value);
  [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hwnd,ref Point value);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd,StringBuilder value,int size);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd,StringBuilder value,int size);
  [DllImport("user32.dll",SetLastError=true)] static extern IntPtr SendMessageTimeout(IntPtr hwnd,uint message,UIntPtr wParam,IntPtr lParam,uint flags,uint timeout,out UIntPtr result);
  public static string Check(int pid) {
    string error="waiting";
    EnumWindows((hwnd,arg)=>{
      uint owner;GetWindowThreadProcessId(hwnd,out owner);if(owner!=(uint)pid||!IsWindowVisible(hwnd))return true;
      var name=new StringBuilder(256);GetClassName(hwnd,name,256);if(name.ToString()=="Ghost"){error="Windows created a ghost window";return false;}
      // WebView2 also owns tiny helper HWNDs; only Bimu is the app surface.
      var title=new StringBuilder(256);GetWindowText(hwnd,title,256);if(title.ToString()!="Bimu")return true;
      UIntPtr result;if(SendMessageTimeout(hwnd,0,UIntPtr.Zero,IntPtr.Zero,2,250,out result)==IntPtr.Zero){error="UI message thread did not respond";return false;}
      Rect outer,client;Point origin=new Point();
      if(!GetWindowRect(hwnd,out outer)||!GetClientRect(hwnd,out client)||!ClientToScreen(hwnd,ref origin)){error="Could not read the actual native frame";return false;}
      if(Math.Abs(origin.X-outer.Left)>2||Math.Abs(origin.Y-outer.Top)>2||Math.Abs((outer.Right-outer.Left)-client.Right)>2||Math.Abs((outer.Bottom-outer.Top)-client.Bottom)>2){error=String.Format("Unexpected native frame: origin offset {0},{1}; outer {2}x{3}, client {4}x{5}",origin.X-outer.Left,origin.Y-outer.Top,outer.Right-outer.Left,outer.Bottom-outer.Top,client.Right,client.Bottom);return false;}
      error="ok";return false;
    },IntPtr.Zero);return error;
  }
}
'@
$oldArguments=$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$appProcess=$null;$testProcess=$null
# Hosted runners are elevated. Runtime 150+ intentionally ignores WEBVIEW2_*
# arguments there; an app-scoped machine policy is supported and restored below.
$policy='HKLM:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$policyName='Bimu.exe';$policyHadValue=$false;$policyPrevious=$null;$policyChanged=$false
$stdout=Join-Path $env:RUNNER_TEMP 'bimu-native-smoke.out'
$stderr=Join-Path $env:RUNNER_TEMP 'bimu-native-smoke.err'
try {
  # Debugging is enabled only for this isolated CI process, never by the app.
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9225 --remote-debugging-address=127.0.0.1'
  if($env:GITHUB_ACTIONS -eq 'true'){
    if(Test-Path $policy){$key=Get-Item $policy;$policyHadValue=$key.GetValueNames() -contains $policyName;if($policyHadValue){$policyPrevious=$key.GetValue($policyName)}}
    New-Item -Path $policy -Force | Out-Null
    New-ItemProperty -Path $policy -Name $policyName -Value $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -PropertyType String -Force | Out-Null
    $policyChanged=$true
  }
  $appProcess=Start-Process -FilePath (Resolve-Path $Executable).Path -PassThru
  $testProcess=Start-Process -FilePath (Get-Command node).Source -ArgumentList 'tests/windows-native.browser.mjs' -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
  $deadline=[DateTime]::UtcNow.AddSeconds(70);$samples=0;$misses=0
  while(!$testProcess.HasExited) {
    if([DateTime]::UtcNow -gt $deadline){throw 'Native window test timed out.'}
    if($appProcess.HasExited){throw 'Bimu exited during native window testing.'}
    $state=[BimuNativeWindow]::Check($appProcess.Id)
    if($state -eq 'ok'){$samples++;$misses=0}
    elseif($state -ne 'waiting'){$misses++;if($misses -ge 3){throw $state}}
    Start-Sleep -Milliseconds 200
    $testProcess.Refresh();$appProcess.Refresh()
  }
  $testProcess.WaitForExit()
  Get-Content $stdout -ErrorAction SilentlyContinue
  Get-Content $stderr -ErrorAction SilentlyContinue
  if($testProcess.ExitCode -ne 0){throw "Native WebView2 test failed: $($testProcess.ExitCode)"}
  if($samples -lt 3){throw 'Did not observe a responsive, undecorated native window.'}
  Write-Host "PASS: $samples native frame and UI-heartbeat samples."
} finally {
  Get-Content $stdout -ErrorAction SilentlyContinue
  Get-Content $stderr -ErrorAction SilentlyContinue
  if($testProcess -and !$testProcess.HasExited){Stop-Process -Id $testProcess.Id -Force -ErrorAction SilentlyContinue}
  if($appProcess -and !$appProcess.HasExited){Stop-Process -Id $appProcess.Id -Force -ErrorAction SilentlyContinue}
  if($policyChanged){
    if($policyHadValue){Set-ItemProperty -Path $policy -Name $policyName -Value $policyPrevious}
    else{Remove-ItemProperty -Path $policy -Name $policyName -ErrorAction SilentlyContinue}
  }
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=$oldArguments
}
