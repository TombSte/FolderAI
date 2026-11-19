# Native file interoperability

Discovery evidence for DA-003–004, RF-024–026 and RT-005.

## macOS probe

Run `swift docs/platform-probes/macos-file-interop.swift` from the repository root in a standard macOS desktop session. The probe writes synthetic files and uses a private pasteboard.

To check interprocess delivery, compile the same source with `swiftc -warnings-as-errors` and run the resulting binary with `--two-processes`. On APFS the second process received both paths in their original order, including the accents and spaces.

On 2026-11-19, AppKit on macOS 26.5.1 Apple Silicon round-tripped two items through a private named `NSPasteboard`, using `writeObjects` with native file URLs and `readObjects` with `urlReadingFileURLsOnly`. Each item advertised `public.file-url`. The paths contained spaces and accented characters, and the sequence contained one regular file and one directory. The shell sandbox denied the pasteboard write. The same probe succeeded in the desktop session outside the shell sandbox.

`URLResourceValues.contentType` returned `public.plain-text` for the synthetic TXT file and `public.png` for the synthetic PNG. `NSWorkspace` resolved TextEdit and Preview as the defaults on this host and returned them in their respective handler lists. It found Terminal by bundle identifier. Bidirectional Finder, Desktop and app transfers still require desktop scenarios. [Apple NSWorkspace](https://developer.apple.com/documentation/appkit/nsworkspace)

### Terminal working directory

Compile [macos-terminal.swift](macos-terminal.swift) with `swiftc -parse-as-library -warnings-as-errors` and run the resulting binary in a macOS desktop session. The 2026-11-19 probe passed the temporary directory to `NSWorkspace.open` as a file URL. It opened a separate Terminal instance, and `lsof` confirmed that the login shell used the directory with spaces and an accented character as its working directory. The probe asks AppKit to skip adding the launch to recent items. It checks that the returned Terminal process was new before terminating it at the end of the probe. Reuse of an existing instance and alternate terminal apps remain unverified.

## Adapter design to validate

- Resolve candidate apps per native content type. For multiple selected files, offer apps compatible with every selected type. Store explicit preferences only in FolderAI, keyed by native content type and a native app identifier. Offer an explicit reset to the system association. If an app becomes unavailable or incompatible, report the fallback and use the system handler when available.
- Read real file URLs on macOS. Finder's Move Item Here action uses Option–Command–V. Keep FolderAI cut state tied to the pasteboard generation and verified item IDs. Confirm the native operation result before removing a cut marker. Test OS-origin transfers with Finder and Desktop in both directions. [Finder shortcuts](https://support.apple.com/en-gb/102650)
- On Windows, validate Unicode `CF_HDROP` for ordinary filesystem paths and the Shell drop-effect formats for move, copy and acknowledgment. Treat the effect requested by the source as a preference. Report the effect that actually completed. Test Explorer, Desktop and a representative app in both directions before claiming compatibility. [Microsoft Shell clipboard formats](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard)
- Launch only a supported terminal adapter with a directory argument or native working-directory field. Discover Terminal by bundle identifier on macOS. Windows Terminal documents a starting-directory option and treats semicolons as command delimiters. Test Windows arguments with spaces, Unicode and literal delimiter characters before enabling that adapter. [Windows Terminal command-line arguments](https://learn.microsoft.com/en-us/windows/terminal/command-line-arguments)
