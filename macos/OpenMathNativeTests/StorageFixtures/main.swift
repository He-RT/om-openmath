import Darwin
import Foundation
import SQLite3
@main struct StorageFixtures {
  static func modify(_ url:URL,_ sql:String) async throws {
    try await withCheckedThrowingContinuation { (continuation:CheckedContinuation<Void,any Error>) in
      DispatchQueue.global(qos:.utility).async {
        do { let db=try SQLiteDatabase(url:url);try db.statement(sql);try db.close();continuation.resume() }
        catch { continuation.resume(throwing:error) }
      }
    }
  }
  static func main() async throws {
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    if CommandLine.arguments.count>2 {
      let stage=CommandLine.arguments[2]
      if stage=="check-recovery" {
        do { _ = try await StorageService.open(paths:.init(root:root,channel:.preview));fatalError("interrupted generation guessed") }
        catch StorageError.recoveryRequired { print("recovery_required");return }
      }
      if stage=="inspect" || stage=="hold-open" {
        let (store,info)=try await StorageService.open(paths:.init(root:root,channel:.preview))
        if stage=="hold-open" { FileHandle.standardOutput.write(Data("lease-held\n".utf8));raise(SIGSTOP) }
        else { FileHandle.standardOutput.write(try JSONEncoder().encode(info));FileHandle.standardOutput.write(Data("\n".utf8)) }
        try await store.close();return
      }
      let faults=StorageFaults { point in
        if point==stage { FileHandle.standardOutput.write(Data((point+"\n").utf8));raise(SIGSTOP) }
      }
      let (store,_)=try await StorageService.open(paths:.init(root:root,channel:.preview),faults:faults)
      try await store.close();return
    }
    precondition(om_fixture_install_sync_probe()==SQLITE_OK)
    let paths=NativeStoragePaths(root:root.appendingPathComponent("valid",isDirectory:true),channel:.preview)
    let (service,opened)=try await StorageService.open(paths:paths)
    precondition(opened.runtime.journalMode=="wal" && opened.runtime.synchronous==2 && opened.runtime.fullfsync==1)
    let header=try await service.readLibraryHeader()
    precondition(header==opened.identity)
    do { _ = try await StorageService.open(paths:paths);fatalError("second writer admitted") }
    catch StorageError.inUse { }
    let id=UUID().uuidString.lowercased()
    let document=try await service.openDocument(id)
    precondition(document.identity.kind=="document" && document.identity.documentID==id)
    do { _ = try await service.openDocument(id);fatalError("duplicate document writer") }
    catch StorageError.inUse { }
    try await service.close()
    let (reopened,again)=try await StorageService.open(paths:paths)
    precondition(again.identity==header)
    try await reopened.close()
    precondition(om_fixture_unknown_fcntl_calls()==0)
    FileHandle.standardError.write(Data("sync calls=\(om_fixture_fullsync_calls()) success=\(om_fixture_fullsync_successes()) unknown=\(om_fixture_unknown_fcntl_calls())\n".utf8))
    precondition(om_fixture_fullsync_calls()>0 && om_fixture_fullsync_successes()>0)
    print("Actual SQLite",opened.runtime.version,opened.runtime.vfs,"F_FULLFSYNC calls",om_fixture_fullsync_calls(),"success",om_fixture_fullsync_successes(),"barriers",om_fixture_barrier_calls())
    // An absent selector with a real existing DB must request recovery, never initialize empty.
    let selector=paths.root.appendingPathComponent("Library/active.json")
    let selectorBytes=try Data(contentsOf:selector)
    try FileManager.default.removeItem(at:selector)
    do { _ = try await StorageService.open(paths:paths);fatalError("unselected library silently rebuilt") }
    catch StorageError.recoveryRequired { }
    try selectorBytes.write(to:selector)
    let future=root.appendingPathComponent("future",isDirectory:true)
    try ManagedFiles.directory(future)
    let metadata=future.appendingPathComponent("store.json")
    let bytes=Data("{\"format_version\":99,\"installation_id\":\"00000000-0000-0000-0000-000000000000\",\"channel\":\"preview\"}".utf8)
    try bytes.write(to:metadata)
    do { _ = try await StorageService.open(paths:.init(root:future,channel:.preview));fatalError("future root accepted") }
    catch StorageError.unsupportedVersion { }
    let unchanged=try Data(contentsOf:metadata)
    precondition(unchanged==bytes)
    precondition(!FileManager.default.fileExists(atPath:future.appendingPathComponent("store.lock").path))
    let readonly=root.appendingPathComponent("readonly",isDirectory:true)
    try ManagedFiles.directory(readonly);precondition(chmod(readonly.path,0o500)==0)
    do { _ = try await StorageService.open(paths:.init(root:readonly,channel:.preview));fatalError("readonly root accepted") }
    catch StorageError.inaccessible { }
    precondition(chmod(readonly.path,0o700)==0)
    let linked=root.appendingPathComponent("linked",isDirectory:true)
    try FileManager.default.createSymbolicLink(at:linked,withDestinationURL:paths.root)
    do { _ = try await StorageService.open(paths:.init(root:linked,channel:.preview));fatalError("symlink root accepted") }
    catch StorageError.unsafePath { }
    let full=root.appendingPathComponent("full",isDirectory:true)
    let failing=StorageFaults { point in if point=="before_database_open" {om_fixture_fail_writes(1)} }
    do { _ = try await StorageService.open(paths:.init(root:full,channel:.preview),faults:failing);fatalError("ENOSPC returned success") }
    catch StorageError.sqlite(let code,_) { precondition(code & 255 == SQLITE_FULL || code & 255 == SQLITE_IOERR) }
    om_fixture_fail_writes(0)
    precondition(!FileManager.default.fileExists(atPath:full.appendingPathComponent("Library/active.json").path))
    do { _ = try await StorageService.open(paths:.init(root:full,channel:.preview));fatalError("failed generation overwritten") }
    catch StorageError.recoveryRequired { }
    // Future DB versions are rejected before configuring a writer, without altering DB bytes.
    let futureDB=paths.root.appendingPathComponent("Library/Generations/000001/library.sqlite")
    try await modify(futureDB,"PRAGMA user_version=99")
    let before=try Data(contentsOf:futureDB)
    do { _ = try await StorageService.open(paths:paths);fatalError("future database accepted") }
    catch StorageError.unsupportedVersion { }
    let after=try Data(contentsOf:futureDB);precondition(before==after)
    let unknown=root.appendingPathComponent("unknown-sync",isDirectory:true)
    let failedSync=StorageFaults { point in if point=="before_bootstrap_commit" { om_fixture_fail_fullsync(1) } }
    do { _ = try await StorageService.open(paths:.init(root:unknown,channel:.preview),faults:failedSync);fatalError("postcommit sync failed but acknowledged") }
    catch StorageError.unknownCommit { }
    om_fixture_fail_fullsync(0)
    precondition(!FileManager.default.fileExists(atPath:unknown.appendingPathComponent("Library/active.json").path))
    do { _ = try await StorageService.open(paths:.init(root:unknown,channel:.preview));fatalError("unknown commit silently reinitialized") }
    catch StorageError.recoveryRequired { }
    let sentinel=root.appendingPathComponent("old-config.toml")
    let sentinelData=Data("[legacy]\ncredential=never-read-fixture\n".utf8)
    try sentinelData.write(to:sentinel);precondition(chmod(sentinel.path,0)==0)
    let (isolated,_)=try await StorageService.open(paths:.init(root:root.appendingPathComponent("new-channel"),channel:.preview))
    try await isolated.close();precondition(chmod(sentinel.path,0o600)==0)
    let oldData=try Data(contentsOf:sentinel);precondition(oldData==sentinelData)
    print("Bootstrap/readback/reopen, locks, missing selector, future root/DB, unwritable/symlink root, real SQLite ENOSPC/postcommit sync unknown and legacy sentinel passed")
  }
}
