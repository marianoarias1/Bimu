# Pill, previews and saved devices (0.55.18)

The floating design is optional; the existing notch remains the default. Both designs render the same pages, file previews and device settings. The compact pill contains only eyes, clock and connection/battery status. Music uses a closed perimeter comet, with the existing leading head and fading tail. Position is stored per monitor in normalized work-area coordinates; a fixed native envelope contains all hover/settings layouts so opening does not wait for a native resize.

## Saved devices and revocation

A versioned device book is protected with current-user DPAPI and written atomically. Migration preserves the exact legacy PSK and original main/client role. A damaged new store never falls back to a stale legacy credential. Successful migration removes the old single-device file. Views returned to JavaScript contain display metadata only, never keys. Names and LAN advertisements are hints, not authentication.

Each connection authenticates with Noise using its saved PSK. New clients advertise/choose sync-v6 and negotiate a control intent before clipboard or catalog exchange. Legacy sync-v5 peers continue to synchronize; management controls and remote thumbnail requests require both versions to support v6. Manual-address pairing can initially use v5 and upgrades on a capable automatic reconnection.

Disconnect pauses a relationship and keeps its credential. The authenticated disconnect receipt pauses the other endpoint too. An explicit Connect can resume either direction; it never displaces an active connection without disconnecting it first. Favorite/last-used ranking applies while idle. Only one device may synchronize at a time; the original code-generating role still owns any configured shared transfer limit even when TCP roles reverse.

Forget marks a relationship revoked before closing its data channel. An encrypted revocation-only tombstone retains the ability to notify an offline peer and acknowledge retries; it never authorizes clipboard/catalog traffic. Revoked records cannot be selected or favored. Pending notifications remain visible until delivered. Offline or busy PCs cannot be notified instantaneously. Revocation receipts are retained to handle lost acknowledgements; both endpoints must be upgraded for bilateral notification. The book is bounded (32 live devices, 128 records including receipts).

## Previews

Shell thumbnails are requested lazily for visible items by their current saved IDs; peers never supply local paths. Images copied as pixels keep their existing preview. File previews use Windows thumbnail providers, so codec availability varies; the current icon is the fallback. The local cache is bounded and keyed by path, size and modification time. Two workers limit concurrent extraction. Network preview requests are capability-gated, bounded, and scoped to the authenticated session, item revision and request token. PNG responses have bounded dimensions/bytes. Thumbnail preparation does not block the synchronization loop.

## Validation and manual release gate

Unit/integration checks cover negative monitor origins/DPI, the closed comet, explicit new-pairing navigation, credential migration/failure behavior, revocation precedence and pause/resume over real encrypted channels. Windows CI additionally exercises a real Shell thumbnail and the production React UI in Edge with native IPC fixtures. The UI fixtures validate rendering/navigation, not actual multi-PC behavior.

Before merge, test both designs with themes/custom colors and real image files/captures; move the pill across monitors/DPI and restart. With at least three updated PCs, switch between saved peers, change the favorite, disconnect and resume, forget an online peer, and deliver a pending forget after an offline peer returns. Check that only the selected connection shares data and that existing transfer-limit behavior remains intact. Also test basic sync with a previous-version peer, with management controls unavailable until it is updated. Recheck lyrics/volume, cleanup layout and all notch placements. The Store startup path needs a real MSIX installation.
