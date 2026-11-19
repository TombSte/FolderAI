#Requires -Version 7.0
[CmdletBinding()]
param([string]$FixtureParent = [IO.Path]::GetTempPath())

$ErrorActionPreference = 'Stop'
if (!$IsWindows -or [Environment]::OSVersion.Version.Build -lt 22000 -or
    [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64' -or
    [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'X64') {
    throw 'Run this probe in x64 PowerShell 7 on Windows 11 x64.'
}

Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;

public static class FolderAIWindowsProbe
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern bool GetVolumePathNameW(string file, StringBuilder root, uint size);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern bool GetVolumeInformationW(string root, IntPtr label, uint labelSize,
        out uint serial, out uint componentMax, out uint flags, StringBuilder format, uint formatSize);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern bool MoveFileExW(string source, string destination, uint flags);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern bool ReplaceFileW(string destination, string staged, string backup,
        uint flags, IntPtr reserved1, IntPtr reserved2);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    private static extern bool CreateHardLinkW(string link, string target, IntPtr reserved);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    private static extern bool CreateSymbolicLinkW(string link, string target, uint flags);

    private static bool Check(string label, bool passed)
    {
        Console.WriteLine("{0} {1}", passed ? "PASS" : "FAIL", label);
        return passed;
    }

    private static int Publish(string source, string destination) =>
        MoveFileExW(source, destination, 0) ? 0 : Marshal.GetLastWin32Error();

    private static int Replace(string destination, string staged) =>
        ReplaceFileW(destination, staged, null, 0, IntPtr.Zero, IntPtr.Zero) ? 0 : Marshal.GetLastWin32Error();

    private static bool Matches(string path, string expected) => File.Exists(path) && File.ReadAllText(path) == expected;

    public static bool Run(string directory)
    {
        var root = new StringBuilder(32768);
        var format = new StringBuilder(256);
        if (!GetVolumePathNameW(directory, root, (uint)root.Capacity) ||
            !GetVolumeInformationW(root.ToString(), IntPtr.Zero, 0, out _, out uint maximum,
                out uint flags, format, (uint)format.Capacity))
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());

        Console.WriteLine("OBS filesystem={0}; component maximum={1}; flags=0x{2:X}", format, maximum, flags);

        string name = Path.Combine(directory, "a caf\u00e9.txt");
        File.WriteAllText(name, "old");
        bool passed = Check("create Unicode name with spaces", Matches(name, "old"));
        Console.WriteLine("OBS case variant resolves: {0}", File.Exists(Path.Combine(directory, "A CAF\u00c9.TXT")));
        Console.WriteLine("OBS decomposed Unicode resolves: {0}", File.Exists(Path.Combine(directory, "a cafe\u0301.txt")));

        string hardLink = Path.Combine(directory, "hard-link");
        if (CreateHardLinkW(hardLink, name, IntPtr.Zero))
        {
            File.WriteAllText(hardLink, "via hard link");
            passed &= Check("hard link writes through to original", Matches(name, "via hard link"));
        }
        else Console.WriteLine("OBS hard link creation declined: Win32 {0}", Marshal.GetLastWin32Error());
        string symlink = Path.Combine(directory, "symlink");
        const uint AllowUnprivilegedCreate = 2;
        if (CreateSymbolicLinkW(symlink, Path.GetFileName(name), AllowUnprivilegedCreate))
        {
            passed &= Check("read through symlink", File.ReadAllText(symlink) == File.ReadAllText(name));
            File.Delete(name);
            passed &= Check("dangling symlink still has a link target", new FileInfo(symlink).LinkTarget != null);
        }
        else Console.WriteLine("OBS symlink creation declined: Win32 {0}", Marshal.GetLastWin32Error());

        string missing = Path.Combine(directory, "missing");
        string unused = Path.Combine(directory, "unused");
        int missingMove = Publish(missing, unused);
        Console.WriteLine("OBS move of missing file: Win32 {0}", missingMove);
        passed &= Check("missing source reports file not found and leaves destination absent",
            missingMove == 2 && !File.Exists(unused));
        string tree = Path.Combine(directory, "tree");
        string child = Path.Combine(tree, "child");
        Directory.CreateDirectory(child);
        string descendant = Path.Combine(child, "moved");
        int descendantMove = Publish(tree, descendant);
        Console.WriteLine("OBS move into descendant: Win32 {0}", descendantMove);
        passed &= Check("move into a descendant leaves the tree intact",
            descendantMove != 0 && Directory.Exists(child) && !Directory.Exists(descendant));

        string first = Path.Combine(directory, "first");
        string second = Path.Combine(directory, "second");
        string destination = Path.Combine(directory, "published");
        File.WriteAllText(first, "first");
        File.WriteAllText(second, "second");
        var firstMove = Task.Run(() => Publish(first, destination));
        var secondMove = Task.Run(() => Publish(second, destination));
        Task.WaitAll(firstMove, secondMove);
        Console.WriteLine("OBS exclusive publish results: Win32 {0}, {1}", firstMove.Result, secondMove.Result);
        const int FileExists = 80, AlreadyExists = 183;
        passed &= Check("exclusive publication keeps one winner and preserves the loser",
            (firstMove.Result == 0 && (secondMove.Result == FileExists || secondMove.Result == AlreadyExists) &&
                Matches(destination, "first") && Matches(second, "second")) ||
            (secondMove.Result == 0 && (firstMove.Result == FileExists || firstMove.Result == AlreadyExists) &&
                Matches(destination, "second") && Matches(first, "first")));

        string staged = Path.Combine(directory, "staged");
        string current = File.ReadAllText(destination);
        File.WriteAllText(staged, "replacement");
        int blocked;
        using (File.Open(destination, FileMode.Open, FileAccess.Read, FileShare.ReadWrite))
            blocked = Replace(destination, staged);
        Console.WriteLine("OBS replacement with destination opened without delete sharing: Win32 {0}", blocked);
        passed &= Check("failed replacement preserves the old and staged data",
            blocked != 0 && Matches(destination, current) && Matches(staged, "replacement"));
        int replaced = Replace(destination, staged);
        Console.WriteLine("OBS explicit replacement: Win32 {0}", replaced);
        passed &= Check("explicit replacement publishes staged data",
            replaced == 0 && Matches(destination, "replacement") && !File.Exists(staged));
        return passed;
    }
}
'@

$parent = (Get-Item -LiteralPath $FixtureParent).FullName
$fixture = [IO.Path]::Combine($parent, 'FolderAI-probe-' + [guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($fixture)
try {
    $principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    $elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    Write-Host "OBS OS=$([Environment]::OSVersion.Version); architecture=x64; elevated=$elevated"
    $passed = [FolderAIWindowsProbe]::Run($fixture)
} finally {
    [IO.Directory]::Delete($fixture, $true)
}
if (!$passed) { exit 1 }
