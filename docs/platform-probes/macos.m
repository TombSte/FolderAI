#import <Foundation/Foundation.h>
#include <copyfile.h>
#include <dispatch/dispatch.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <unistd.h>

static int failures;

static void check(const char *label, BOOL passed) {
    printf("%s %s\n", passed ? "PASS" : "FAIL", label);
    failures += !passed;
}

static BOOL create(const char *path, char byte) {
    int fd = open(path, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (fd == -1) return NO;
    BOOL written = write(fd, &byte, 1) == 1;
    return close(fd) == 0 && written;
}

static BOOL reads(int fd, char expected) {
    char byte;
    return fd >= 0 && pread(fd, &byte, 1, 0) == 1 && byte == expected;
}

static BOOL path_reads(const char *path, char expected) {
    int fd = open(path, O_RDONLY);
    if (fd == -1) return NO;
    BOOL matches = reads(fd, expected);
    close(fd);
    return matches;
}

static BOOL path_names_handle(const char *path, int fd) {
    struct stat named, held;
    return fd >= 0 && lstat(path, &named) == 0 && fstat(fd, &held) == 0 &&
        named.st_dev == held.st_dev && named.st_ino == held.st_ino;
}

static void probe_swap(void) {
    check("create swap fixtures", create("swap-target", 'o') && create("swap-stage", 'n'));
    int target = open("swap-target", O_RDONLY);
    int staged = open("swap-stage", O_RDONLY);
    check("missing staged file leaves the approved target untouched",
          renamex_np("swap-missing", "swap-target", RENAME_SWAP) == -1 && errno == ENOENT &&
          path_names_handle("swap-target", target) && path_reads("swap-target", 'o'));
    check("swap keeps the displaced file at the staging name",
          renamex_np("swap-stage", "swap-target", RENAME_SWAP) == 0 &&
          path_names_handle("swap-stage", target) && path_reads("swap-stage", 'o') &&
          path_names_handle("swap-target", staged) && path_reads("swap-target", 'n'));
    if (target >= 0) close(target);
    if (staged >= 0) close(staged);

    check("prepare target substitution fixture",
          create("race-target", 'a') && create("race-stage", 'b') && create("race-intruder", 'c'));
    int approved = open("race-target", O_RDONLY);
    check("substitute a new target between approval and publication",
          path_names_handle("race-target", approved) && rename("race-target", "race-original") == 0 &&
          rename("race-intruder", "race-target") == 0);
    check("swap succeeds against a foreign target and retains all three objects",
          renamex_np("race-stage", "race-target", RENAME_SWAP) == 0 &&
          !path_names_handle("race-stage", approved) &&
          path_names_handle("race-original", approved) && path_reads("race-original", 'a') &&
          path_reads("race-target", 'b') && path_reads("race-stage", 'c'));
    if (approved >= 0) close(approved);

    check("prepare in-place modification fixture",
          create("edit-target", 'x') && create("edit-stage", 'y'));
    int edited = open("edit-target", O_RDWR);
    check("contents can change under a matching file identity",
          edited >= 0 && path_names_handle("edit-target", edited) &&
          pwrite(edited, "z", 1, 0) == 1 && path_names_handle("edit-target", edited));
    check("the displaced object retains the intervening edit",
          renamex_np("edit-stage", "edit-target", RENAME_SWAP) == 0 &&
          path_names_handle("edit-stage", edited) && path_reads("edit-stage", 'z') &&
          path_reads("edit-target", 'y'));
    if (edited >= 0) close(edited);
}

static void probe_space(void) {
    struct statfs filesystem;
    if (statfs(".", &filesystem) != 0 ||
        (uint64_t)filesystem.f_blocks * filesystem.f_bsize > 160 * 1024 * 1024) {
        check("use a disposable filesystem of at most 160 MiB", NO);
        return;
    }
    check("create and capture the destination before staging", create("space-target", 'o'));
    int original = open("space-target", O_RDONLY);
    int stage = open("space-stage", O_WRONLY | O_CREAT | O_EXCL, 0600);
    int error = stage < 0 ? errno : 0;
    off_t bytes = 0;
    char buffer[65536];
    while (stage >= 0 && error == 0 && bytes < 192 * 1024 * 1024) {
        arc4random_buf(buffer, sizeof(buffer));
        ssize_t written = write(stage, buffer, sizeof(buffer));
        if (written == -1) {
            error = errno;
        } else if (written == 0) {
            error = EIO;
        } else {
            bytes += written;
        }
    }
    if (stage >= 0 && close(stage) != 0 && error == 0) error = errno;
    printf("OBS staging wrote %lld bytes before errno %d\n", (long long)bytes, error);
    check("actual ENOSPC interrupts a partially written staged copy", bytes > 0 && error == ENOSPC);
    check("discard only the partial staging file and preserve the destination",
          unlink("space-stage") == 0 &&
          path_names_handle("space-target", original) && path_reads("space-target", 'o'));
    if (original >= 0) close(original);
}

static void probe_directory_anchor(void) {
    check("create captured destination and unrelated directory",
          mkdir("anchor-target", 0700) == 0 && mkdir("anchor-unrelated", 0700) == 0 &&
          create("anchor-target/staged", 's') && create("anchor-unrelated/untouched", 'u'));
    int destination = open("anchor-target", O_RDONLY | O_DIRECTORY | O_NOFOLLOW);
    check("capture the native destination directory",
          path_names_handle("anchor-target", destination));
    check("replace the visible destination path with a symlink",
          rename("anchor-target", "anchor-original") == 0 &&
          symlink("anchor-unrelated", "anchor-target") == 0 &&
          !path_names_handle("anchor-target", destination));
    if (destination >= 0) {
        struct stat absent;
        check("descriptor-relative publication cannot follow the substituted parent path",
              renameatx_np(destination, "staged", destination, "published", RENAME_EXCL) == 0 &&
              path_names_handle("anchor-original", destination) &&
              path_reads("anchor-original/published", 's') &&
              lstat("anchor-unrelated/published", &absent) == -1 && errno == ENOENT &&
              path_reads("anchor-unrelated/untouched", 'u'));
        close(destination);
    }
}

static void probe_names_and_permissions(void) {
    check("create a Unicode name with spaces", create("a caf\xc3\xa9.txt", 'a'));
    struct stat original, alternate;
    if (stat("a caf\xc3\xa9.txt", &original) == 0) {
        printf("OBS case variant resolves to original: %s\n",
               stat("A CAF\xc3\x89.TXT", &alternate) == 0 &&
               original.st_ino == alternate.st_ino ? "yes" : "no");
        printf("OBS decomposed Unicode resolves to original: %s\n",
               stat("a cafe\xcc\x81.txt", &alternate) == 0 &&
               original.st_ino == alternate.st_ino ? "yes" : "no");
    }

    long name_max = pathconf(".", _PC_NAME_MAX);
    printf("OBS reported component limit: %ld\n", name_max);
    if (name_max > 0) {
        char *name = malloc((size_t)name_max + 2);
        if (name == NULL) {
            check("allocate maximum-length name fixture", NO);
        } else {
            memset(name, 'x', (size_t)name_max + 1);
            name[name_max] = '\0';
            check("create at the reported ASCII component limit", create(name, 'l'));
            name[name_max] = 'x';
            name[name_max + 1] = '\0';
            int fd = open(name, O_WRONLY | O_CREAT | O_EXCL, 0600);
            check("reject an ASCII component past the limit", fd == -1 && errno == ENAMETOOLONG);
            if (fd != -1) close(fd);
            free(name);
        }
    }

    int fd = open("missing", O_RDONLY);
    check("missing file returns ENOENT", fd == -1 && errno == ENOENT);
    if (fd != -1) close(fd);

    BOOL protected = create("protected", 'p') && chmod("protected", 0000) == 0;
    fd = open("protected", O_RDONLY);
    int access_error = fd == -1 ? errno : 0;
    printf("OBS chmod 000 succeeded: %s; subsequent read open errno: %d\n",
           protected ? "yes" : "no", access_error);
    check("mode 000 denies a read by an unprivileged user",
          protected && fd == -1 && access_error == EACCES);
    if (fd != -1) close(fd);
    if (protected) check("restore fixture permissions", chmod("protected", 0600) == 0);
}

static void probe_links(void) {
    check("create link target", create("target", 't'));
    struct stat target, alias;
    int hard_link_error = link("target", "hard-link") == 0 ? 0 : errno;
    printf("OBS hard link errno: %d\n", hard_link_error);
    check("hard link shares the immediate target inode",
          hard_link_error == 0 && stat("target", &target) == 0 &&
          stat("hard-link", &alias) == 0 && target.st_ino == alias.st_ino);
    check("inspect symlink without following and read through it",
          symlink("target", "symlink") == 0 && lstat("symlink", &alias) == 0 &&
          S_ISLNK(alias.st_mode) && path_reads("symlink", 't'));
    check("dangling link remains visible to lstat",
          symlink("missing", "dangling") == 0 && lstat("dangling", &alias) == 0 &&
          S_ISLNK(alias.st_mode) && stat("dangling", &alias) == -1 && errno == ENOENT);
    check("following a cyclic link returns ELOOP",
          symlink("cycle", "cycle") == 0 && stat("cycle", &alias) == -1 && errno == ELOOP);
    check("rename into a descendant fails and preserves the tree",
          mkdir("tree", 0700) == 0 && mkdir("tree/child", 0700) == 0 &&
          rename("tree", "tree/child/moved") == -1 && errno == EINVAL &&
          stat("tree/child", &alias) == 0 && S_ISDIR(alias.st_mode));
}

static void probe_finder_alias(NSURL *fixture) {
    NSURL *target = [fixture URLByAppendingPathComponent:@"target"];
    NSURL *alias = [fixture URLByAppendingPathComponent:@"finder-alias"];
    NSError *error = nil;
    NSData *bookmark = [target bookmarkDataWithOptions:NSURLBookmarkCreationSuitableForBookmarkFile
                       includingResourceValuesForKeys:nil relativeToURL:nil error:&error];
    BOOL written = bookmark != nil && [NSURL writeBookmarkData:bookmark toURL:alias options:0 error:&error];
    check("write a native Finder alias", written);
    if (!written) return;

    struct stat link_info;
    check("Finder alias is a regular file", lstat(alias.fileSystemRepresentation, &link_info) == 0 &&
          S_ISREG(link_info.st_mode));
    check("rename Finder alias target", rename("target", "renamed-target") == 0);
    NSURL *resolved = [NSURL URLByResolvingAliasFileAtURL:alias
                     options:NSURLBookmarkResolutionWithoutUI | NSURLBookmarkResolutionWithoutMounting
                       error:&error];
    check("resolve Finder alias after target rename", resolved != nil &&
          [resolved.lastPathComponent isEqualToString:@"renamed-target"] &&
          path_reads(resolved.fileSystemRepresentation, 't'));
}

static void probe_cross_volume_source_retained(NSURL *fixture) {
    NSURL *staging = [fixture URLByAppendingPathComponent:@"staged"];
    NSURL *published = [fixture URLByAppendingPathComponent:@"published"];
    const char *source_path = "source-retained/item";
    const char *staging_path = staging.fileSystemRepresentation;
    const char *published_path = published.fileSystemRepresentation;

    check("create the source for the cross-volume staged copy",
          mkdir("source-retained", 0700) == 0 && create(source_path, 'm'));
    int source = open(source_path, O_RDONLY);
    int staged = open(staging_path, O_RDWR | O_CREAT | O_EXCL, 0600);
    BOOL ready = source >= 0 && staged >= 0 &&
        fcopyfile(source, staged, NULL, COPYFILE_DATA) == 0 && fsync(staged) == 0 &&
        reads(staged, 'm') && path_names_handle(source_path, source);
    check("finish staging across volumes and verify both handles", ready);
    BOOL copied = ready && renamex_np(staging_path, published_path, RENAME_EXCL) == 0 &&
        path_names_handle(published_path, staged);
    check("publish the staged object while the source remains", copied && path_reads(source_path, 'm'));

    if (copied) {
        BOOL restricted = chmod("source-retained", 0500) == 0;
        BOOL retained = restricted && unlink(source_path) == -1 && errno == EACCES &&
            path_names_handle(source_path, source) && path_names_handle(published_path, staged) &&
            path_reads(source_path, 'm') && path_reads(published_path, 'm');
        check("denied source removal leaves both identities and contents intact", retained);
        BOOL restored = chmod("source-retained", 0700) == 0;
        struct stat missing;
        check("retry removes the verified source and preserves the published object",
              retained && restored &&
              path_names_handle(source_path, source) && path_names_handle(published_path, staged) &&
              unlink(source_path) == 0 && lstat(source_path, &missing) == -1 && errno == ENOENT &&
              path_names_handle(published_path, staged) && path_reads(published_path, 'm'));
    }
    if (source >= 0) close(source);
    if (staged >= 0) close(staged);
}

static void probe_cross_volume(NSFileManager *files, const char *other_parent, BOOL check_source_removal) {
    NSURL *parent = [NSURL fileURLWithFileSystemRepresentation:other_parent isDirectory:YES relativeToURL:nil];
    NSURL *fixture = [parent URLByAppendingPathComponent:NSUUID.UUID.UUIDString isDirectory:YES];
    NSError *error = nil;
    if (![files createDirectoryAtURL:fixture withIntermediateDirectories:NO
                          attributes:@{NSFilePosixPermissions: @0700} error:&error]) {
        fprintf(stderr, "%s\n", error.localizedDescription.UTF8String);
        check("create cross-volume fixture", NO);
        return;
    }

    struct stat source_info, target_info;
    BOOL different_device = stat(".", &source_info) == 0 &&
        stat(fixture.fileSystemRepresentation, &target_info) == 0 && source_info.st_dev != target_info.st_dev;
    check("fixture directories are on different devices", different_device);
    if (different_device) {
        NSURL *destination = [fixture URLByAppendingPathComponent:@"cross-volume"];
        const char *target = destination.fileSystemRepresentation;
        check("create cross-volume source and existing destination",
              create("cross-volume", 's') && create(target, 'd'));
        check("cross-volume rename returns EXDEV and preserves both files",
              rename("cross-volume", target) == -1 && errno == EXDEV &&
              path_reads("cross-volume", 's') && path_reads(target, 'd'));
        if (check_source_removal) probe_cross_volume_source_retained(fixture);
    }
    check("clean up cross-volume fixture", [files removeItemAtURL:fixture error:&error]);
}

static void probe_replacement(void) {
    check("create old file and completed replacement",
          create("destination", 'o') && create("staged", 'n'));
    check("exclusive rename preserves existing destination and source on collision",
          renamex_np("staged", "destination", RENAME_EXCL) == -1 && errno == EEXIST &&
          path_reads("destination", 'o') && path_reads("staged", 'n'));
    check("rename with a missing staged file preserves destination",
          rename("missing", "destination") == -1 && errno == ENOENT &&
          path_reads("destination", 'o'));
    int previous = open("destination", O_RDONLY);
    check("same-volume replacement publishes new data and preserves existing open handle",
          rename("staged", "destination") == 0 && path_reads("destination", 'n') &&
          reads(previous, 'o'));
    if (previous != -1) close(previous);

    check("create two independent staged files", create("first", '1') && create("second", '2'));
    int outcomes[2];
    int *results = outcomes;
    dispatch_apply(2, DISPATCH_APPLY_AUTO, ^(size_t contender) {
        const char *source = contender == 0 ? "first" : "second";
        results[contender] = renamex_np(source, "published", RENAME_EXCL) == 0 ? 0 : errno;
    });
    printf("OBS concurrent exclusive rename errno: %d, %d\n", outcomes[0], outcomes[1]);
    check("two exclusive publications produce one winner and preserve the losing source",
          (outcomes[0] == 0 && outcomes[1] == EEXIST && path_reads("published", '1') &&
           path_reads("second", '2')) ||
          (outcomes[1] == 0 && outcomes[0] == EEXIST && path_reads("published", '2') &&
           path_reads("first", '1')));
}

static void probe_trash(NSFileManager *files, NSURL *fixture) {
    check("create trash fixture", create("trash-fixture", 'r'));
    NSURL *item = [fixture URLByAppendingPathComponent:@"trash-fixture"];
    NSURL *trashed = nil;
    NSError *error = nil;
    if (![files trashItemAtURL:item resultingItemURL:&trashed error:&error]) {
        printf("OBS trash error: %s (%ld)\n", error.domain.UTF8String, (long)error.code);
        check("native trash and restore", NO);
        return;
    }
    struct stat info;
    check("native trash removed the original and returned a readable item",
          lstat("trash-fixture", &info) == -1 && errno == ENOENT && trashed != nil &&
          path_reads(trashed.fileSystemRepresentation, 'r'));
    BOOL restored = trashed != nil && [files moveItemAtURL:trashed toURL:item error:&error];
    if (trashed != nil && !restored) {
        fprintf(stderr, "Fixture remains at %s\n", trashed.fileSystemRepresentation);
    }
    check("restore the synthetic item using the URL returned by the trash API",
          restored && path_reads("trash-fixture", 'r'));
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        BOOL swap_only = argc == 3 && strcmp(argv[2], "--swap-only") == 0;
        BOOL space_only = argc == 3 && strcmp(argv[2], "--space-only") == 0;
        BOOL anchor_only = argc == 3 && strcmp(argv[2], "--anchor-only") == 0;
        BOOL cross_only = argc == 4 && strcmp(argv[2], "--cross-only") == 0 && argv[3][0] == '/';
        if ((argc != 2 && argc != 3 && !cross_only) || geteuid() == 0 || argv[1][0] != '/' ||
            (argc == 3 && !swap_only && !space_only && !anchor_only && argv[2][0] != '/')) {
            fputs("Usage (standard user): macos-probe /fixture-parent "
                  "[/second-volume-parent | --swap-only | --space-only | --anchor-only "
                  "| --cross-only /second-volume-parent]\n", stderr);
            return 2;
        }
        NSFileManager *files = NSFileManager.defaultManager;
        NSURL *parent = [NSURL fileURLWithFileSystemRepresentation:argv[1] isDirectory:YES
                                                    relativeToURL:nil];
        NSURL *fixture = [parent URLByAppendingPathComponent:NSUUID.UUID.UUIDString isDirectory:YES];
        NSError *error = nil;
        if (![files createDirectoryAtURL:fixture withIntermediateDirectories:NO
                              attributes:@{NSFilePosixPermissions: @0700} error:&error]) {
            fprintf(stderr, "%s\n", error.localizedDescription.UTF8String);
            return 2;
        }
        if (chdir(fixture.fileSystemRepresentation) == 0) {
            if (swap_only) {
                probe_swap();
            } else if (space_only) {
                probe_space();
            } else if (anchor_only) {
                probe_directory_anchor();
            } else if (cross_only) {
                probe_cross_volume(files, argv[3], YES);
            } else {
                probe_names_and_permissions();
                probe_links();
                probe_finder_alias(fixture);
                probe_replacement();
                if (argc == 3) probe_cross_volume(files, argv[2], NO);
                probe_trash(files, fixture);
            }
        } else {
            perror("chdir");
            failures++;
        }
        BOOL cleaned = [files removeItemAtURL:fixture error:&error];
        if (!cleaned) {
            printf("OBS fixture cleanup error: %s (%ld)\n",
                   error.domain.UTF8String, (long)error.code);
        }
        check("clean up fixture directory", cleaned);
        return failures == 0 ? 0 : 1;
    }
}
