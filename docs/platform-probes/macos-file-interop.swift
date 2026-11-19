import AppKit
import UniformTypeIdentifiers

enum ProbeError: Error {
    case pasteboardWriteFailed
    case pasteboardRoundTripMismatch
    case pasteboardReaderFailed
    case missingDefaultHandler(String)
}

func readPaths(from pasteboard: NSPasteboard) -> [String] {
    let urls = pasteboard.readObjects(
        forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]
    ) as? [URL] ?? []
    return urls.map(\.path)
}

func verifySecondProcess(pasteboard: NSPasteboard, paths: [String], reader: String) throws {
    let child = Process()
    child.executableURL = URL(fileURLWithPath: reader)
    child.arguments = ["--read-pasteboard", pasteboard.name.rawValue] + paths
    try child.run()
    child.waitUntilExit()
    guard child.terminationStatus == 0 else { throw ProbeError.pasteboardReaderFailed }
}

func run() throws {
    let manager = FileManager.default
    let root = manager.temporaryDirectory.appendingPathComponent(
        "FolderAI native caffè \(UUID())", isDirectory: true
    )
    try manager.createDirectory(at: root, withIntermediateDirectories: false)
    defer { try? manager.removeItem(at: root) }

    let text = root.appendingPathComponent("prova è.txt")
    let folder = root.appendingPathComponent("cartella con spazi ù", isDirectory: true)
    let png = root.appendingPathComponent("example.png")
    try Data("synthetic".utf8).write(to: text)
    try Data().write(to: png)
    try manager.createDirectory(at: folder, withIntermediateDirectories: false)

    let pasteboard = NSPasteboard.withUniqueName()
    defer { pasteboard.releaseGlobally() }
    pasteboard.clearContents()
    guard pasteboard.writeObjects([text as NSURL, folder as NSURL]) else {
        throw ProbeError.pasteboardWriteFailed
    }
    let paths = [text.path, folder.path]
    guard readPaths(from: pasteboard) == paths else {
        throw ProbeError.pasteboardRoundTripMismatch
    }
    print("Pasteboard:", pasteboard.pasteboardItems?.map { $0.types.map(\.rawValue) } ?? [])
    if CommandLine.arguments.dropFirst().first == "--two-processes" {
        try verifySecondProcess(pasteboard: pasteboard, paths: paths, reader: CommandLine.arguments[0])
    }

    let workspace = NSWorkspace.shared
    for url in [text, png] {
        let type = try url.resourceValues(forKeys: [.contentTypeKey]).contentType?.identifier
        let defaultApp = workspace.urlForApplication(toOpen: url)
        let handlers = workspace.urlsForApplications(toOpen: url)
        if let defaultApp, !handlers.contains(defaultApp) {
            throw ProbeError.missingDefaultHandler(url.pathExtension)
        }
        let bundle = defaultApp.flatMap { Bundle(url: $0)?.bundleIdentifier }
        print("Association:", url.pathExtension, type ?? "none", bundle ?? "none", "handlers:", handlers.count)
    }
    print("Terminal installed:", workspace.urlForApplication(withBundleIdentifier: "com.apple.Terminal") != nil)
}

do {
    if CommandLine.arguments.count == 5 && CommandLine.arguments[1] == "--read-pasteboard" {
        let pasteboard = NSPasteboard(name: NSPasteboard.Name(CommandLine.arguments[2]))
        guard readPaths(from: pasteboard) == Array(CommandLine.arguments.suffix(2)) else {
            throw ProbeError.pasteboardRoundTripMismatch
        }
        print("PASS separate process read both file URLs from the private pasteboard")
    } else {
        try run()
    }
} catch {
    FileHandle.standardError.write(Data("Native interoperability probe failed: \(error)\n".utf8))
    exit(1)
}
