import AppKit

enum ProbeError: Error {
    case terminalMissing, instanceReused, directoryUnobserved
}

func output(_ program: String, _ arguments: [String]) throws -> String {
    let process = Process()
    let pipe = Pipe()
    process.executableURL = URL(fileURLWithPath: program)
    process.arguments = arguments
    process.standardOutput = pipe
    process.standardError = FileHandle.nullDevice
    try process.run()
    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    process.waitUntilExit()
    return String(decoding: data, as: UTF8.self)
}

func openChildDirectories(pid: pid_t, expected: String, depth: Int = 0) throws -> Bool {
    guard depth < 4 else { return false }
    let children = try output("/usr/bin/pgrep", ["-P", String(pid)])
        .split(whereSeparator: \.isWhitespace)
        .compactMap { pid_t($0) }
    for child in children {
        let cwd = try output("/usr/sbin/lsof", ["-a", "-p", String(child), "-d", "cwd", "-Fn"])
        if cwd.split(separator: "\n").contains(where: { $0 == "n" + expected }) { return true }
        if try openChildDirectories(pid: child, expected: expected, depth: depth + 1) { return true }
    }
    return false
}

func shellUsesDirectory(_ path: String) throws -> Bool {
    guard let account = getpwuid(getuid()), let loginShell = account.pointee.pw_shell else {
        return false
    }
    let shell = URL(fileURLWithPath: String(cString: loginShell)).lastPathComponent
    let scopedUsage = try output("/usr/sbin/lsof", ["-Fcf", "+d", path])
    var isLoginShell = false
    for line in scopedUsage.split(separator: "\n") {
        if line.hasPrefix("p") { isLoginShell = false }
        if line.hasPrefix("c") { isLoginShell = line.dropFirst() == shell }
        if line == "fcwd" && isLoginShell { return true }
    }
    return false
}

@main
struct TerminalProbe {
    @MainActor
    static func main() async {
        do {
            let terminalId = "com.apple.Terminal"
            guard let terminal = NSWorkspace.shared.urlForApplication(withBundleIdentifier: terminalId) else {
                throw ProbeError.terminalMissing
            }
            let existing = Set(NSRunningApplication.runningApplications(withBundleIdentifier: terminalId)
                .map(\.processIdentifier))
            let root = FileManager.default.temporaryDirectory
                .appendingPathComponent("FolderAI terminal caffè \(UUID())", isDirectory: true)
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
            defer { try? FileManager.default.removeItem(at: root) }
            let configuration = NSWorkspace.OpenConfiguration()
            configuration.createsNewApplicationInstance = true
            configuration.activates = false
            configuration.addsToRecentItems = false
            configuration.environment = ["ZDOTDIR": root.path, "HISTFILE": "/dev/null"]
            let app = try await NSWorkspace.shared.open([root], withApplicationAt: terminal, configuration: configuration)
            guard !existing.contains(app.processIdentifier) else { throw ProbeError.instanceReused }
            defer {
                if !app.isTerminated { _ = app.forceTerminate() }
            }
            let expected = root.resolvingSymlinksInPath().path
            for _ in 0..<20 {
                if try openChildDirectories(pid: app.processIdentifier, expected: expected) {
                    print("PASS a child of the new Terminal instance had the expected working directory")
                    return
                }
                if !app.isTerminated, try shellUsesDirectory(expected) {
                    print("PASS the login shell used the unique Terminal fixture as its working directory")
                    return
                }
                try await Task.sleep(for: .milliseconds(150))
            }
            let children = try output("/usr/bin/pgrep", ["-P", String(app.processIdentifier)]).split(whereSeparator: \.isWhitespace)
            print("OBS launch running:", !app.isTerminated, "immediate children:", children.count)
            throw ProbeError.directoryUnobserved
        } catch {
            FileHandle.standardError.write(Data("Terminal probe: \(error)\n".utf8))
            exit(1)
        }
    }
}
