import Foundation
import SQLite3

/// Canonical session events and producer counters share one Library transaction. No network,
/// evaluator or tool callback can be called by this queue-confined storage owner.
final class AgentSessionStore {
  static let tables:Set<String>=["agent_sessions","session_events","session_producers","session_operation_claims"]
  private let db:SQLiteDatabase,url:URL,identity:StoreIdentity
  private let index:SessionRecoveryIndex?
  init(db:SQLiteDatabase,url:URL,identity:StoreIdentity,create:Bool=false)throws {
    guard identity.kind=="library",(3...4).contains(identity.storeVersion) else {throw StorageError.unsupportedVersion}
    self.db=db;self.url=url;self.identity=identity
    var present=Set<String>();try db.statement("SELECT name FROM sqlite_schema WHERE type='table'"){present.insert(try SQLColumn.text($0,0))}
    if present.intersection(Self.tables) != Self.tables {
      guard create,present.intersection(Self.tables).isEmpty else {throw StorageError.recoveryRequired}
      try db.transaction {
        try db.statement("CREATE TABLE agent_sessions(session_id TEXT PRIMARY KEY,record BLOB NOT NULL,record_hash TEXT NOT NULL)")
        try db.statement("CREATE TABLE session_events(session_id TEXT NOT NULL,event_id TEXT NOT NULL,event_sequence INTEGER NOT NULL CHECK(event_sequence>0),source_instance TEXT NOT NULL,source_sequence INTEGER NOT NULL CHECK(source_sequence>0),request_hash TEXT NOT NULL,record BLOB NOT NULL,record_hash TEXT NOT NULL,receipt BLOB NOT NULL,receipt_hash TEXT NOT NULL,PRIMARY KEY(session_id,event_id),UNIQUE(session_id,event_sequence),UNIQUE(source_instance,source_sequence),FOREIGN KEY(session_id) REFERENCES agent_sessions(session_id))")
        try db.statement("CREATE TABLE session_producers(source_instance TEXT PRIMARY KEY,last_sequence INTEGER NOT NULL CHECK(last_sequence>0))")
        try db.statement("CREATE TABLE session_operation_claims(store_id TEXT NOT NULL,operation_id TEXT NOT NULL,session_id TEXT NOT NULL,event_id TEXT NOT NULL,PRIMARY KEY(store_id,operation_id),FOREIGN KEY(session_id,event_id) REFERENCES session_events(session_id,event_id))")
      }
    }
    index=identity.storeVersion==4 ? try SessionRecoveryIndex(db:db,create:create) : nil
    let prefix=try pageGlobal(db,limit:16)
    var previous:[String:[NativeSessionEvent]]=[:]
    for record in prefix {
      let e=record.event,events=previous[e.agent_session_id] ?? []
      guard e.event_sequence==UInt64(events.count)+1 else {throw StorageError.corruptIdentity}
      let input=SessionAppendInput(session:e.agent_session_id,eventID:e.event_id,sourceInstance:e.source_instance_id,sourceSequence:e.source_sequence,parentEvent:e.parent_event_id.value,turn:e.agent_turn_id.value,task:e.task_id.value,kind:e.event_kind,payload:Data(e.payload_inline_json.value!.utf8),operation:e.operation_id.value,operationStore:e.operation_store_id.value)
      try SessionSemantics.validate(input,events:events);try verifyResources(input,events:events,reader:db,stored:true)
      previous[e.agent_session_id]=events+[e]
    }
  }
  private func session(_ id:String,reader:SQLiteDatabase)throws->AgentSessionIdentity? {
    var result:AgentSessionIdentity?
    try reader.statement("SELECT record,record_hash FROM agent_sessions WHERE session_id=?",[.text(id)]) {row in
      let bytes=try SQLColumn.data(row,0);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}
      let value=try JSONDecoder().decode(AgentSessionIdentity.self,from:bytes)
      guard value.codecVersion==1,value.id==id,SourceValidation.identity(value.id),!value.createdAt.isEmpty,
        value.documentID.map(SourceValidation.identity) ?? true,(value.title?.utf8.count ?? 0)<=16384,
        (value.documentIDs?.count ?? 0)<=10000,(value.documentIDs ?? []).allSatisfy(SourceValidation.identity) else {throw StorageError.corruptIdentity};result=value
    };return result
  }
  func create(_ id:String,documentID:String?=nil,title:String?=nil,faults:StorageFaults)throws->AgentSessionIdentity {
    guard SourceValidation.identity(id),documentID.map(SourceValidation.identity) ?? true,(title?.utf8.count ?? 0)<=16384 else {throw StorageError.corruptIdentity}
    if let existing=try session(id,reader:db) {
      guard existing.documentID==nil || documentID==nil || existing.owners.contains(documentID!) else {throw StorageError.corruptIdentity}
      try db.confirmCommittedBytes();return existing
    }
    let value=AgentSessionIdentity(codecVersion:1,id:id,createdAt:ISO8601DateFormatter().string(from:Date()),documentID:documentID,title:title.map{String($0.trimmingCharacters(in:.whitespacesAndNewlines).prefix(20)).replacingOccurrences(of:"\n",with:" ")}),bytes=try RecoveryValidation.encode(value)
    try db.transaction {
      try db.statement("INSERT INTO agent_sessions VALUES(?,?,?)",[.text(id),.blob(bytes),.text(KernelHashes.data(bytes))]);try faults.reach("agent_session_before_commit")
    }
    do {try faults.reach("agent_session_committed");let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};guard let actual=try session(id,reader:reader) else {throw StorageError.corruptIdentity};try db.confirmCommittedBytes();return actual}
    catch {throw StorageError.unknownCommit(EIO)}
  }
  /// Called only after the service has verified this document's current history association.
  /// Associate on explicit activation, keeping read-only history browsing free of writes.
  func associate(_ id:String,document:String,title:String?=nil)throws->AgentSessionIdentity {
    try associate([id],document:document)
    guard var value=try session(id,reader:db),(title?.utf8.count ?? 0)<=16384 else {throw StorageError.corruptIdentity}
    // Attachments can create an untitled draft session before its first actual question.
    if value.title?.isEmpty != false,let title,!title.isEmpty {
      value=AgentSessionIdentity(codecVersion:value.codecVersion,id:value.id,createdAt:value.createdAt,documentID:value.documentID,title:String(title.trimmingCharacters(in:.whitespacesAndNewlines).prefix(20)).replacingOccurrences(of:"\n",with:" "),documentIDs:value.documentIDs)
      let bytes=try RecoveryValidation.encode(value)
      try db.transaction {try db.statement("UPDATE agent_sessions SET record=?,record_hash=? WHERE session_id=?",[.blob(bytes),.text(KernelHashes.data(bytes)),.text(id)])};try db.confirmCommittedBytes()
    }
    return value
  }
  /// One metadata transaction for a verified file open; canonical events remain untouched.
  func associate(_ ids:[String],document:String)throws {
    guard SourceValidation.identity(document) else {throw StorageError.corruptIdentity}
    var updates:[(String,Data)]=[]
    for id in Set(ids).sorted() {
      guard let existing=try session(id,reader:db),let original=existing.documentID else {throw StorageError.corruptIdentity}
      if existing.owners.contains(document) {continue}
      guard existing.owners.count<10000 else {throw StorageError.recoveryRequired}
      let owners=existing.owners.union([document]).subtracting([original]).sorted()
      let value=AgentSessionIdentity(codecVersion:existing.codecVersion,id:id,createdAt:existing.createdAt,documentID:original,title:existing.title,documentIDs:owners)
      updates.append((id,try RecoveryValidation.encode(value)))
    }
    guard !updates.isEmpty else {return}
    try db.transaction {
      for (id,bytes) in updates {try db.statement("UPDATE agent_sessions SET record=?,record_hash=? WHERE session_id=?",[.blob(bytes),.text(KernelHashes.data(bytes)),.text(id)])}
    }
    try db.confirmCommittedBytes()
  }
  private func decode(_ row:OpaquePointer)throws->StoredSessionEvent {
    let record=try SQLColumn.data(row,6),receiptBytes=try SQLColumn.data(row,8)
    guard KernelHashes.data(record)==(try SQLColumn.text(row,7)),KernelHashes.data(receiptBytes)==(try SQLColumn.text(row,9)) else {throw StorageError.corruptIdentity}
    let event=try JSONDecoder().decode(NativeSessionEvent.self,from:record);try SessionValidation.event(event)
    let receipt=try JSONDecoder().decode(SessionEventReceipt.self,from:receiptBytes)
    guard event.agent_session_id==(try SQLColumn.text(row,0)),event.event_id==(try SQLColumn.text(row,1)),
      Int64(event.event_sequence)==sqlite3_column_int64(row,2),event.source_instance_id==(try SQLColumn.text(row,3)),Int64(event.source_sequence)==sqlite3_column_int64(row,4),
      receipt.storeID==identity.storeID,receipt.session==event.agent_session_id,receipt.eventID==event.event_id,receipt.eventSequence==event.event_sequence,
      receipt.sourceInstance==event.source_instance_id,receipt.sourceSequence==event.source_sequence,receipt.requestHash==(try SQLColumn.text(row,5)),receipt.recordHash==KernelHashes.data(record) else {throw StorageError.corruptIdentity}
    let reconstructed=SessionAppendInput(session:event.agent_session_id,eventID:event.event_id,sourceInstance:event.source_instance_id,sourceSequence:event.source_sequence,
      parentEvent:event.parent_event_id.value,turn:event.agent_turn_id.value,task:event.task_id.value,kind:event.event_kind,payload:Data(event.payload_inline_json.value!.utf8),operation:event.operation_id.value,operationStore:event.operation_store_id.value)
    guard receipt.requestHash==(try SessionValidation.requestHash(reconstructed)) else {throw StorageError.corruptIdentity}
    try index?.verify(event)
    return .init(event:event,receipt:receipt)
  }
  private static let columns="session_id,event_id,event_sequence,source_instance,source_sequence,request_hash,record,record_hash,receipt,receipt_hash"
  /// Maintenance scans every committed record without retaining entire long conversations.
  func validateAll()throws {
    var previous:[String:UInt64]=[:]
    try db.statement("SELECT \(Self.columns) FROM session_events ORDER BY session_id,event_sequence") { [self] row in
      let event=try decode(row).event
      guard event.event_sequence==(previous[event.agent_session_id] ?? 0)+1 else {throw StorageError.corruptIdentity}
      previous[event.agent_session_id]=event.event_sequence
      let input=SessionAppendInput(session:event.agent_session_id,eventID:event.event_id,sourceInstance:event.source_instance_id,sourceSequence:event.source_sequence,parentEvent:event.parent_event_id.value,turn:event.agent_turn_id.value,task:event.task_id.value,kind:event.event_kind,payload:Data(event.payload_inline_json.value!.utf8),operation:event.operation_id.value,operationStore:event.operation_store_id.value)
      if let index {
        let related=try index.related(input) { [self] id in guard let actual=try read(event.agent_session_id,event:id,reader:db) else {throw StorageError.corruptIdentity};return actual.event}.filter{$0.event_sequence<event.event_sequence}
        try SessionSemantics.validate(input,events:related);try verifyResources(input,events:related,reader:db,stored:true)
      }
    }
    try db.statement("SELECT source_instance,last_sequence FROM session_producers") {row in
      var count:Int64=0,maximum:Int64=0
      try self.db.statement("SELECT count(*),coalesce(max(source_sequence),0) FROM session_events WHERE source_instance=?",[.text(try SQLColumn.text(row,0))]) {count=sqlite3_column_int64($0,0);maximum=sqlite3_column_int64($0,1)}
      guard count==maximum,maximum==sqlite3_column_int64(row,1) else {throw StorageError.corruptIdentity}
    }
  }
  func read(_ session:String,event:String,reader:SQLiteDatabase)throws->StoredSessionEvent? {
    var value:StoredSessionEvent?
    try reader.statement("SELECT \(Self.columns) FROM session_events WHERE session_id=? AND event_id=?",[.text(session),.text(event)]) { [self] row in value=try decode(row)}
    return value
  }
  func query(_ session:String,event:String)throws->StoredSessionEvent? {
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    let actual=try read(session,event:event,reader:reader);try db.confirmCommittedBytes();return actual
  }
  func append(_ input:SessionAppendInput,resource:BlobReference?=nil,faults:StorageFaults)throws->SessionAppendReceipt {
    try SessionValidation.input(input);let requestHash=try SessionValidation.requestHash(input)
    if let old=try query(input.session,event:input.eventID) {
      guard old.receipt.requestHash==requestHash else {throw StorageError.idempotencyConflict};return .init(stored:old,inserted:false)
    }
    try db.transaction {
      guard try session(input.session,reader:db) != nil else {throw StorageError.corruptIdentity}
      if let parent=input.parentEvent {guard try read(input.session,event:parent,reader:db) != nil else {throw SessionLedgerError.invalidTransition}}
      var previous:Int64=0;try db.statement("SELECT last_sequence FROM session_producers WHERE source_instance=?",[.text(input.sourceInstance)]) {previous=sqlite3_column_int64($0,0)}
      guard previous>=0,previous<Int64(HostSerial.maximum),Int64(input.sourceSequence)==previous+1 else {throw SessionLedgerError.sequenceGap}
      let previousEvents:[NativeSessionEvent]
      if let index {previousEvents=try index.related(input) { [self] id in guard let actual=try read(input.session,event:id,reader:db) else {throw StorageError.corruptIdentity};return actual.event}}
      else {let all=try recovery(input.session,reader:db);guard all.complete else {throw StorageError.recoveryRequired};previousEvents=all.events}
      try SessionSemantics.validate(input,events:previousEvents)
      if let resource {try BlobReferences.insertWithinTransaction(resource,db:db)}
      try verifyResources(input,events:previousEvents,reader:db)
      var claim:SessionToolAdmission?
      var runChildClaim:SessionOperationTarget?
      if input.kind == .task_transition,let transition=try SessionRunSemantics.transition(input.payload),transition.kind == .run_child_admitted {
        guard let group=previousEvents.first(where:{$0.event_id==transition.groupAdmission}),let index=transition.childIndex else {throw SessionLedgerError.invalidTransition}
        let admission=try SessionValidation.payload(SessionToolAdmission.self,group),target=try SessionRunSemantics.target(admission,index:index)
        var existing=false
        try db.statement("SELECT event_id FROM session_operation_claims WHERE store_id=? AND operation_id=?",[.text(target.storeID),.text(target.operationID)]) {_ in existing=true}
        guard !existing else {throw SessionLedgerError.executionAlreadyClaimed};runChildClaim=target
      }
      if input.kind == .tool_admitted {
        let admitted=try JSONDecoder().decode(SessionToolAdmission.self,from:input.payload)
        var existing:(String,String)?
        try db.statement("SELECT session_id,event_id FROM session_operation_claims WHERE store_id=? AND operation_id=?",[.text(admitted.target.storeID),.text(admitted.target.operationID)]) {existing=(try SQLColumn.text($0,0),try SQLColumn.text($0,1))}
        if let existing {
          guard admitted.run==nil else {throw SessionLedgerError.reconciliationRequired}
          guard let originalClaim=try read(existing.0,event:existing.1,reader:db),originalClaim.event.event_kind == .tool_admitted,
            try SessionValidation.payload(SessionToolAdmission.self,originalClaim.event).run==nil else {throw SessionLedgerError.executionAlreadyClaimed}
          var actual:DocumentEffect?
          try db.statement("SELECT record,record_hash FROM document_effect_events WHERE source_store_id=? AND operation_id=?",[.text(admitted.target.storeID),.text(admitted.target.operationID)]) {row in
            let bytes=try SQLColumn.data(row,0);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)),actual==nil else {throw StorageError.corruptIdentity}
            actual=try JSONDecoder().decode(DocumentEffect.self,from:bytes)
          }
          guard let actual else {throw SessionLedgerError.reconciliationRequired}
          _ = try SessionSemantics.effectOutcome(actual,target:admitted.target)
        } else {claim=admitted}
      }
      var last:Int64=0;try db.statement("SELECT coalesce(max(event_sequence),0) FROM session_events WHERE session_id=?",[.text(input.session)]){last=sqlite3_column_int64($0,0)}
      guard last>=0,last<Int64(HostSerial.maximum) else {throw StorageError.corruptIdentity}
      let event=NativeSessionEvent(record_type:"session_event",event_version:1,agent_session_id:input.session,event_id:input.eventID,event_sequence:UInt64(last+1),source_instance_id:input.sourceInstance,source_sequence:input.sourceSequence,parent_event_id:.init(input.parentEvent),agent_turn_id:.init(input.turn),task_id:.init(input.task),event_kind:input.kind,payload_hash:KernelHashes.data(input.payload),payload_inline_json:.init(String(data:input.payload,encoding:.utf8)),payload_blob_hash:.init(nil),operation_id:.init(input.operation),operation_store_id:.init(input.operationStore),created_at:ISO8601DateFormatter().string(from:Date()))
      try SessionValidation.event(event);let bytes=try RecoveryValidation.encode(event)
      guard bytes.count<=4*1024*1024 else {throw StorageError.corruptIdentity}
      let receipt=SessionEventReceipt(storeID:identity.storeID,session:input.session,eventID:input.eventID,eventSequence:event.event_sequence,sourceInstance:input.sourceInstance,sourceSequence:input.sourceSequence,requestHash:requestHash,recordHash:KernelHashes.data(bytes)),receiptBytes=try RecoveryValidation.encode(receipt)
      try db.statement("INSERT INTO session_events VALUES(?,?,?,?,?,?,?,?,?,?)",[.text(input.session),.text(input.eventID),.integer(last+1),.text(input.sourceInstance),.integer(Int64(input.sourceSequence)),.text(requestHash),.blob(bytes),.text(KernelHashes.data(bytes)),.blob(receiptBytes),.text(KernelHashes.data(receiptBytes))])
      try index?.insert(event)
      if let claim {
        try db.statement("INSERT INTO session_operation_claims VALUES(?,?,?,?)",[.text(claim.target.storeID),.text(claim.target.operationID),.text(input.session),.text(input.eventID)])
      }
      if let claim=runChildClaim {
        try db.statement("INSERT INTO session_operation_claims VALUES(?,?,?,?)",[.text(claim.storeID),.text(claim.operationID),.text(input.session),.text(input.eventID)])
      }
      try db.statement("INSERT INTO session_producers VALUES(?,?) ON CONFLICT(source_instance) DO UPDATE SET last_sequence=excluded.last_sequence",[.text(input.sourceInstance),.integer(Int64(input.sourceSequence))])
      try faults.reach("session_event_before_commit")
      try faults.reach("session_event_before_commit."+input.kind.rawValue)
    }
    do {try faults.reach("session_event_committed");try faults.reach("session_event_committed."+input.kind.rawValue);guard let actual=try query(input.session,event:input.eventID),actual.receipt.requestHash==requestHash else {throw StorageError.corruptIdentity};try faults.reach("session_event_confirmed");return .init(stored:actual,inserted:true)}
    catch {throw StorageError.unknownCommit(EIO)}
  }
  func page(_ session:String,after:UInt64=0,limit:Int=16)throws->[StoredSessionEvent] {
    guard after<=HostSerial.maximum,limit>0,limit<=32 else {throw StorageError.corruptIdentity}
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    return try events(session,after:after,limit:limit,reader:reader)
  }
  private func events(_ session:String,after:UInt64,limit:Int,reader:SQLiteDatabase)throws->[StoredSessionEvent] {
    var output:[StoredSessionEvent]=[],bytes=0,full=false
    try reader.statement("SELECT \(Self.columns) FROM session_events WHERE session_id=? AND event_sequence>? ORDER BY event_sequence LIMIT ?",[.text(session),.integer(Int64(after)),.integer(Int64(limit))]) { [self] row in
      guard !full else {return};let count=Int(sqlite3_column_bytes(row,6));guard count>0,count<=4*1024*1024 else {throw StorageError.corruptIdentity}
      if !output.isEmpty && bytes+count>4*1024*1024 {full=true;return};output.append(try decode(row));bytes+=count
    };return output
  }
  private func pageGlobal(_ reader:SQLiteDatabase,limit:Int)throws->[StoredSessionEvent] {
    var result:[StoredSessionEvent]=[]
    try reader.statement("SELECT \(Self.columns) FROM session_events ORDER BY rowid LIMIT ?",[.integer(Int64(limit))]) { [self] row in result.append(try decode(row))};return result
  }
  struct RecoveryEvents {let events:[NativeSessionEvent];let complete:Bool}
  func recovery(_ session:String,reader:SQLiteDatabase)throws->RecoveryEvents {
    var all:[NativeSessionEvent]=[],offset:UInt64=0,bytes=0
    while all.count<4096 {
      let page=try events(session,after:offset,limit:min(32,4096-all.count),reader:reader)
      if page.isEmpty {return .init(events:all,complete:true)}
      for record in page {
        guard record.event.event_sequence==offset+1 else {throw StorageError.corruptIdentity}
        bytes+=record.event.payload_inline_json.value!.utf8.count;guard bytes<=16*1024*1024 else {return .init(events:all,complete:false)}
        let e=record.event
        let input=SessionAppendInput(session:e.agent_session_id,eventID:e.event_id,sourceInstance:e.source_instance_id,sourceSequence:e.source_sequence,parentEvent:e.parent_event_id.value,turn:e.agent_turn_id.value,task:e.task_id.value,kind:e.event_kind,payload:Data(e.payload_inline_json.value!.utf8),operation:e.operation_id.value,operationStore:e.operation_store_id.value)
        try SessionSemantics.validate(input,events:all);try verifyResources(input,events:all,reader:reader,stored:true)
        all.append(record.event);offset=record.event.event_sequence
      }
    }
    return .init(events:all,complete:false)
  }
  private func verifyResources(_ input:SessionAppendInput,events:[NativeSessionEvent],reader:SQLiteDatabase,stored:Bool=false)throws {
    if input.kind == .recovery,let record=try SessionSemantics.effectRecord(input.payload) {
      var matched=false
      try reader.statement("SELECT o.byte_length FROM blob_refs r JOIN blob_objects o USING(blob_hash) WHERE r.owner_kind='context' AND r.owner_id=? AND r.blob_hash=?",[.text(input.eventID),.text(record.blobHash)]) {row in
        guard sqlite3_column_int64(row,0)==Int64(record.byteLength) else {throw StorageError.corruptIdentity};matched=true
      };guard matched else {throw BlobError.missing}
    }
    if input.kind == .recovery,let record=try SessionSemantics.readonlyRecord(input.payload) {
      var matched=false
      try reader.statement("SELECT o.byte_length FROM blob_refs r JOIN blob_objects o USING(blob_hash) WHERE r.owner_kind='context' AND r.owner_id=? AND r.blob_hash=?",[.text(input.eventID),.text(record.blobHash)]) {row in
        guard sqlite3_column_int64(row,0)==Int64(record.byteLength) else {throw StorageError.corruptIdentity};matched=true
      };guard matched else {throw BlobError.missing}
    }
    if stored,input.kind == .tool_admitted {
      let admission=try JSONDecoder().decode(SessionToolAdmission.self,from:input.payload)
      var actual:StoredSessionEvent?
      try reader.statement("SELECT session_id,event_id FROM session_operation_claims WHERE store_id=? AND operation_id=?",[.text(admission.target.storeID),.text(admission.target.operationID)]) { [self] row in
        actual=try read(SQLColumn.text(row,0),event:SQLColumn.text(row,1),reader:reader)
      }
      guard let actual,actual.event.event_kind == .tool_admitted,
        try SessionValidation.payload(SessionToolAdmission.self,actual.event).target==admission.target else {throw StorageError.corruptIdentity}
    }
    if input.kind == .tool_result {
      let result=try JSONDecoder().decode(SessionToolResult.self,from:input.payload)
      guard let admitted=events.first(where:{$0.event_id==result.admissionEvent}) else {throw SessionLedgerError.invalidTransition}
      let admission=try SessionValidation.payload(SessionToolAdmission.self,admitted)
      var effect:DocumentEffect?
      try reader.statement("SELECT record,record_hash,effect_hash FROM document_effect_events WHERE source_store_id=? AND event_id=?",[.text(admission.target.storeID),.text(result.sourceEventID)]) {row in
        let bytes=try SQLColumn.data(row,0);guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}
        let value=try JSONDecoder().decode(DocumentEffect.self,from:bytes);try RecoveryValidation.effect(value)
        let stableHash=try value.stableHash
        guard stableHash==(try SQLColumn.text(row,2)),result.sourceEffectHash==stableHash,result.rawReceiptHash==value.rawReceiptHash else {throw SessionLedgerError.wrongSourceEffect};effect=value
      }
      guard let effect,try SessionSemantics.effectOutcome(effect,target:admission.target)==result.outcome else {throw SessionLedgerError.wrongSourceEffect}
    }
    if input.kind == .task_transition {
      if let run=try SessionRunSemantics.transition(input.payload) {
        try verifyRunResources(input,run:run,events:events,reader:reader,stored:stored);return
      }
      let lifecycle=try JSONDecoder().decode(SessionRequestLifecycle.self,from:input.payload)
      let hash:String?,length:UInt64?,owner:String?
      if let plan=lifecycle.plan {hash=plan.wireBlobHash;length=plan.wireByteLength;owner="request-"+input.session+"-"+plan.contextID}
      else if let transition=lifecycle.transition,let response=transition.responseBlobHash {hash=response;length=transition.responseByteLength;owner="response-"+input.session+"-"+transition.contextID}
      else {return}
      guard let hash,let length,let owner,SourceValidation.identity(owner) else {throw StorageError.corruptIdentity}
      var matched=false
      try reader.statement("SELECT o.byte_length FROM blob_refs r JOIN blob_objects o USING(blob_hash) WHERE r.owner_kind='context' AND r.owner_id=? AND r.blob_hash=?",[.text(owner),.text(hash)]) {row in
        guard sqlite3_column_int64(row,0)==Int64(length) else {throw StorageError.corruptIdentity};matched=true
      };guard matched else {throw BlobError.missing}
    }
  }
  func recover(_ session:String,cursor:SessionRecoveryCursor?=nil,limit:Int=16)throws->SessionRecoveryPage {
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    guard try self.session(session,reader:reader) != nil else {throw StorageError.corruptIdentity}
    if index != nil {return try recoverIndexed(session,cursor:cursor,limit:limit,reader:reader)}
    let snapshot=try recovery(session,reader:reader);return try SessionSemantics.recover(session,events:snapshot.events,complete:snapshot.complete)
  }
  private func recoverIndexed(_ session:String,cursor:SessionRecoveryCursor?,limit:Int,reader:SQLiteDatabase)throws->SessionRecoveryPage {
    guard limit>0,limit<=32 else {throw StorageError.corruptIdentity}
    var latest:Int64=0;try reader.statement("SELECT coalesce(max(event_sequence),0) FROM session_events WHERE session_id=?",[.text(session)]) {latest=sqlite3_column_int64($0,0)}
    guard latest>=0,latest<=Int64(HostSerial.maximum) else {throw StorageError.corruptIdentity}
    if let cursor {guard cursor.session==session,cursor.storeID==identity.storeID,cursor.generation==identity.generation,cursor.upperSequence<=UInt64(latest),cursor.afterToolSequence<=cursor.upperSequence,cursor.afterRequestSequence<=cursor.upperSequence else {throw StorageError.corruptIdentity}}
    let upper=cursor?.upperSequence ?? UInt64(latest)
    let view=try SessionRecoveryIndex(db:reader,create:false)
    let fetch:(String)throws->NativeSessionEvent={ [self] id in
      guard let value=try read(session,event:id,reader:reader) else {throw StorageError.corruptIdentity}
      let e=value.event,input=SessionAppendInput(session:e.agent_session_id,eventID:e.event_id,sourceInstance:e.source_instance_id,sourceSequence:e.source_sequence,parentEvent:e.parent_event_id.value,turn:e.agent_turn_id.value,task:e.task_id.value,kind:e.event_kind,payload:Data(e.payload_inline_json.value!.utf8),operation:e.operation_id.value,operationStore:e.operation_store_id.value)
      let related=try view.related(input) { [self] other in guard let actual=try read(session,event:other,reader:reader) else {throw StorageError.corruptIdentity};return actual.event}
      let earlier=related.filter{$0.event_sequence<e.event_sequence}
      try SessionSemantics.validate(input,events:earlier);try verifyResources(input,events:earlier,reader:reader,stored:true)
      return e
    }
    let toolRows=try view.tools(session:session,upper:upper,after:cursor?.afterToolSequence ?? 0,afterID:cursor?.afterToolID ?? "",limit:limit+1)
    let requestRows=try view.requests(session:session,upper:upper,after:cursor?.afterRequestSequence ?? 0,limit:limit+1)
    var tools:[RecoveredTool]=[],requests:[RecoveredRequest]=[]
    for row in toolRows.prefix(limit) {
      let proposal=try fetch(row.proposal),end=try SessionValidation.payload(SessionAssistantEnd.self,proposal)
      guard proposal.event_sequence==row.sequence,proposal.event_kind == .assistant_end,end.calls.contains(where:{$0.callID==row.call}) else {throw StorageError.corruptIdentity}
      var target:SessionOperationTarget?,phase=RecoveredToolPhase.tool_not_started,resultID:String?,runSummary:RecoveredRunSummary?
      if let admissionID=row.admission {
        let admission=try fetch(admissionID),payload=try SessionValidation.payload(SessionToolAdmission.self,admission)
        guard payload.callID==row.call,payload.proposalEvent==row.proposal else {throw StorageError.corruptIdentity};target=payload.target;phase = .tool_outcome_unknown
        if payload.run != nil {
          let snapshot=try runSnapshot(admission:admission,view:view,fetch:fetch,upper:upper)
          phase=SessionRunSemantics.recoveredPhase(snapshot);runSummary=snapshot.summary
          resultID=snapshot.transition?.phase != nil ? snapshot.latest.event_id:nil
        }
        let children=try view.children(session:session,parent:admissionID,fetch:fetch)
        if let result=children.first(where:{$0.event_kind == .tool_result}) {let value=try SessionValidation.payload(SessionToolResult.self,result);phase=value.outcome == .partial ? .partial : .completed;resultID=result.event_id}
      }
      let readonly=try view.readonlyResults(session:session,parent:row.proposal,fetch:fetch).filter{$0.event_sequence<=upper}
      for event in readonly {
        if let record=try SessionSemantics.effectRecord(event),record.intent.tool_call_id==row.call {
          guard record.target==target,record.admissionEvent==row.admission else {throw StorageError.corruptIdentity}
          let effectPhase=try SessionSemantics.effectPhase(record)
          if !(effectPhase == .tool_outcome_unknown && [.completed,.partial].contains(phase)) {phase=effectPhase;resultID=event.event_id}
        }
        if let record=try SessionSemantics.readonlyRecord(event),record.intent.tool_call_id==row.call {
          guard row.admission==nil,resultID==nil else {throw StorageError.corruptIdentity}
          phase=try SessionSemantics.readonlyPhase(record);resultID=event.event_id
        }
      }
      tools.append(.init(session:session,proposalEvent:row.proposal,callID:row.call,admissionEvent:row.admission,phase:phase,target:target,resultEvent:resultID,run:runSummary))
    }
    for row in requestRows.prefix(limit) {
      let plan=try fetch(row.plan);guard plan.event_sequence==row.sequence,try SessionValidation.payload(SessionRequestLifecycle.self,plan).plan?.contextID==row.context else {throw StorageError.corruptIdentity}
      var chain=[plan],last=plan
      for _ in 0..<4 {
        let children=try view.children(session:session,parent:last.event_id,fetch:fetch)
        guard children.count<=1 else {throw StorageError.corruptIdentity}
        guard let next=children.first else {break};chain.append(next);last=next
      }
      guard last.event_id==row.latest else {throw StorageError.corruptIdentity}
      let recovered=try SessionSemantics.recover(session,events:chain,complete:true)
      guard recovered.requests.count==1 else {throw StorageError.corruptIdentity};requests+=recovered.requests
    }
    let complete=toolRows.count<=limit && requestRows.count<=limit
    let next=complete ? nil : SessionRecoveryCursor(session:session,storeID:identity.storeID,generation:identity.generation,upperSequence:upper,afterToolSequence:tools.isEmpty ? cursor?.afterToolSequence ?? 0 : toolRows[min(toolRows.count,limit)-1].sequence,afterToolID:tools.last?.callID ?? cursor?.afterToolID ?? "",afterRequestSequence:requests.isEmpty ? cursor?.afterRequestSequence ?? 0 : requestRows[min(requestRows.count,limit)-1].sequence)
    return .init(tools:tools,requests:requests,lastEventSequence:UInt64(latest),complete:complete,nextCursor:next)
  }
  private func verifyRunResources(_ input:SessionAppendInput,run:SessionRunTransition,events:[NativeSessionEvent],reader:SQLiteDatabase,stored:Bool)throws {
    guard let group=events.first(where:{$0.event_id==run.groupAdmission}) else {throw SessionLedgerError.invalidTransition}
    let admission=try SessionValidation.payload(SessionToolAdmission.self,group)
    if stored,run.kind == .run_child_admitted {
      let target=try SessionRunSemantics.target(admission,index:run.childIndex!)
      var found=false
      try reader.statement("SELECT session_id,event_id FROM session_operation_claims WHERE store_id=? AND operation_id=?",[.text(target.storeID),.text(target.operationID)]) {row in
        guard try SQLColumn.text(row,0)==input.session,try SQLColumn.text(row,1)==input.eventID else {throw StorageError.corruptIdentity};found=true
      };guard found else {throw StorageError.corruptIdentity}
    }
    if run.kind == .run_child_result {
      guard let original=run.original,let index=run.childIndex else {throw SessionLedgerError.invalidTransition}
      let target=try SessionRunSemantics.target(admission,index:index)
      var found=false
      try reader.statement("SELECT record,record_hash,effect_hash FROM document_effect_events WHERE source_store_id=? AND event_id=?",[.text(target.storeID),.text(original.sourceEventID)]) {row in
        let bytes=try SQLColumn.data(row,0)
        guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}
        let effect=try JSONDecoder().decode(DocumentEffect.self,from:bytes);try RecoveryValidation.effect(effect)
        _ = try SessionSemantics.effectOutcome(effect,target:target)
        let stableHash=try effect.stableHash
        let actualReceipt=try JSONDecoder().decode(NativeKernelReceipt.self,from:effect.payload)
        guard try stableHash==SQLColumn.text(row,2),stableHash==original.sourceEffectHash,
          effect.rawReceiptHash==original.rawReceiptHash,
          try RecoveryValidation.encode(original.receipt)==RecoveryValidation.encode(actualReceipt) else {throw SessionLedgerError.wrongSourceEffect};found=true
      };guard found else {throw SessionLedgerError.wrongSourceEffect}
    }
  }
  private func runSnapshot(admission group:NativeSessionEvent,view:SessionRecoveryIndex,fetch:(String)throws->NativeSessionEvent,upper:UInt64=HostSerial.maximum)throws->SessionRunSnapshot {
    let admission=try SessionValidation.payload(SessionToolAdmission.self,group)
    guard let plan=admission.run,group.event_kind == .tool_admitted else {throw SessionLedgerError.invalidTransition}
    var latest=group,finished=false
    for _ in 0..<(plan.children.count*3+8) {
      let children=try view.children(session:group.agent_session_id,parent:latest.event_id,fetch:fetch).filter{$0.event_sequence<=upper}
      guard children.count<=1 else {throw StorageError.corruptIdentity}
      guard let next=children.first else {finished=true;break}
      let nextRun=try SessionRunSemantics.transition(next)
      guard next.event_kind == .tool_started || nextRun?.groupAdmission==group.event_id else {throw StorageError.corruptIdentity}
      latest=next
    }
    guard finished else {throw StorageError.corruptIdentity}
    let t=try SessionRunSemantics.transition(latest)
    return .init(admissionEvent:group,admission:admission,latest:latest,transition:t,progress:t?.progress ?? .init(plan:plan))
  }
  func runSnapshot(_ session:String,admission:String)throws->SessionRunSnapshot {
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    let view=try SessionRecoveryIndex(db:reader,create:false)
    let fetch:(String)throws->NativeSessionEvent={ [self] id in
      guard let value=try read(session,event:id,reader:reader) else {throw StorageError.corruptIdentity}
      let e=value.event,input=SessionAppendInput(session:e.agent_session_id,eventID:e.event_id,sourceInstance:e.source_instance_id,sourceSequence:e.source_sequence,parentEvent:e.parent_event_id.value,turn:e.agent_turn_id.value,task:e.task_id.value,kind:e.event_kind,payload:Data(e.payload_inline_json.value!.utf8),operation:e.operation_id.value,operationStore:e.operation_store_id.value)
      let earlier=try view.related(input) { [self] other in guard let actual=try read(session,event:other,reader:reader) else {throw StorageError.corruptIdentity};return actual.event}.filter{$0.event_sequence<e.event_sequence}
      try SessionSemantics.validate(input,events:earlier);try verifyResources(input,events:earlier,reader:reader,stored:true);return e
    }
    let result=try runSnapshot(admission:fetch(admission),view:view,fetch:fetch)
    try db.confirmCommittedBytes();return result
  }
  func list(after:String?=nil,limit:Int=16)throws->[SessionListRecord] {
    guard limit>0,limit<=32 else {throw StorageError.corruptIdentity}
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()};var records:[SessionListRecord]=[]
    try reader.statement("SELECT a.session_id,coalesce(max(e.event_sequence),0) FROM agent_sessions a LEFT JOIN session_events e USING(session_id) WHERE a.session_id>? GROUP BY a.session_id ORDER BY a.session_id LIMIT ?",[.text(after ?? ""),.integer(Int64(limit))]) { [self] row in
      let id=try SQLColumn.text(row,0),count=sqlite3_column_int64(row,1)
      guard count>=0,count<=Int64(HostSerial.maximum),let actual=try self.session(id,reader:reader) else {throw StorageError.corruptIdentity}
      records.append(.init(id:id,createdAt:actual.createdAt,eventCount:UInt64(count)))
    };return records
  }
  /// Read compact verified session identities only; never scan tool payloads or activate a task.
  func conversations(document:String?=nil)throws->[SessionConversationSummary] {
    guard document.map(SourceValidation.identity) ?? true else {throw StorageError.corruptIdentity}
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    var records:[SessionConversationSummary]=[]
    try reader.statement("SELECT session_id FROM agent_sessions") { [self] row in
      guard let value=try session(SQLColumn.text(row,0),reader:reader),let owner=value.documentID,document==nil || value.owners.contains(document!) else {return}
      records.append(.init(id:value.id,documentID:owner,title:value.title?.isEmpty==false ? value.title!:"新对话",createdAt:value.createdAt,documents:value.owners))
    }
    return records.sorted {($0.createdAt,$0.id)>($1.createdAt,$1.id)}
  }
  /// Indexed immutable claim lookup, restricted to the actual document/store supplied by native.
  func operationClaim(document:String,store:String,operation:String)throws->SessionOperationClaim? {
    guard SourceValidation.identity(document),SourceValidation.identity(store),SourceValidation.identity(operation) else {throw StorageError.corruptIdentity}
    let reader=try SQLiteDatabase(url:url,readonly:true);defer {try? reader.close()}
    var original:(String,String)?
    try reader.statement("SELECT session_id,event_id FROM session_operation_claims WHERE store_id=? AND operation_id=?",[.text(store),.text(operation)]) {row in
      guard original==nil else {throw StorageError.corruptIdentity};original=(try SQLColumn.text(row,0),try SQLColumn.text(row,1))
    }
    guard let original else {return nil}
    guard let actual=try read(original.0,event:original.1,reader:reader) else {throw StorageError.corruptIdentity}
    let event=actual.event
    let target:SessionOperationTarget,group:String?
    if event.event_kind == .tool_admitted {
      let admission=try SessionValidation.payload(SessionToolAdmission.self,event);target=admission.target;group=admission.run==nil ? nil:event.event_id
    } else {
      guard let child=try SessionRunSemantics.transition(event),child.kind == .run_child_admitted,let index=child.childIndex,
        let parent=try read(original.0,event:child.groupAdmission,reader:reader) else {throw StorageError.corruptIdentity}
      let admission=try SessionValidation.payload(SessionToolAdmission.self,parent.event)
      target=try SessionRunSemantics.target(admission,index:index);group=nil
    }
    try SessionValidation.target(target)
    guard target.documentID==document,target.storeID==store,target.operationID==operation else {throw SessionLedgerError.wrongSourceEffect}
    try db.confirmCommittedBytes()
    return .init(event:actual,target:target,runAdmission:group)
  }

}
