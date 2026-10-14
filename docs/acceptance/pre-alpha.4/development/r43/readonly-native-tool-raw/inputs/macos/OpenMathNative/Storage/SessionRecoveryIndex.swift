import Foundation
import SQLite3

/// Transactional pointers into immutable canonical events. They are checked against the
/// original record whenever used; they cannot invent a result or discard unread history.
final class SessionRecoveryIndex {
  static let tables:Set<String>=["session_event_links","session_tool_index","session_request_index"]
  private let db:SQLiteDatabase
  init(db:SQLiteDatabase,create:Bool)throws {
    self.db=db
    var present=Set<String>();try db.statement("SELECT name FROM sqlite_schema WHERE type='table'"){present.insert(try SQLColumn.text($0,0))}
    if present.intersection(Self.tables) != Self.tables {
      guard create,present.intersection(Self.tables).isEmpty else {throw StorageError.recoveryRequired}
      try db.transaction {
        try db.statement("CREATE TABLE session_event_links(session_id TEXT NOT NULL,event_id TEXT NOT NULL,event_sequence INTEGER NOT NULL,event_kind TEXT NOT NULL,parent_event_id TEXT,tool_count INTEGER NOT NULL CHECK(tool_count>=0 AND tool_count<=32),request_count INTEGER NOT NULL CHECK(request_count IN (0,1)),PRIMARY KEY(session_id,event_id),FOREIGN KEY(session_id,event_id) REFERENCES session_events(session_id,event_id))")
        try db.statement("CREATE INDEX session_event_parent ON session_event_links(session_id,parent_event_id,event_kind,event_sequence)")
        try db.statement("CREATE TABLE session_tool_index(session_id TEXT NOT NULL,call_id TEXT NOT NULL,proposal_event TEXT NOT NULL,proposal_sequence INTEGER NOT NULL,admission_event TEXT,PRIMARY KEY(session_id,call_id),FOREIGN KEY(session_id,proposal_event) REFERENCES session_events(session_id,event_id),FOREIGN KEY(session_id,admission_event) REFERENCES session_events(session_id,event_id))")
        try db.statement("CREATE INDEX session_tool_order ON session_tool_index(session_id,proposal_sequence,call_id)")
        try db.statement("CREATE TABLE session_request_index(session_id TEXT NOT NULL,context_id TEXT NOT NULL,plan_event TEXT NOT NULL,plan_sequence INTEGER NOT NULL,latest_event TEXT NOT NULL,PRIMARY KEY(session_id,context_id),FOREIGN KEY(session_id,plan_event) REFERENCES session_events(session_id,event_id),FOREIGN KEY(session_id,latest_event) REFERENCES session_events(session_id,event_id))")
        try db.statement("CREATE INDEX session_request_order ON session_request_index(session_id,plan_sequence)")
        // Explicit staging migration from Library 3; original event/receipt bytes stay intact.
        try db.statement("SELECT record,record_hash FROM session_events ORDER BY session_id,event_sequence") {row in
          let bytes=try SQLColumn.data(row,0)
          guard KernelHashes.data(bytes)==(try SQLColumn.text(row,1)) else {throw StorageError.corruptIdentity}
          let event=try JSONDecoder().decode(NativeSessionEvent.self,from:bytes);try SessionValidation.event(event)
          try self.insert(event)
        }
      }
    }
    let eventCount=try db.integer("SELECT count(*) FROM session_events"),linkCount=try db.integer("SELECT count(*) FROM session_event_links")
    let calls=try db.integer("SELECT coalesce(sum(tool_count),0) FROM session_event_links"),plans=try db.integer("SELECT coalesce(sum(request_count),0) FROM session_event_links")
    guard eventCount==linkCount,calls==(try db.integer("SELECT count(*) FROM session_tool_index")),plans==(try db.integer("SELECT count(*) FROM session_request_index")) else {throw StorageError.corruptIdentity}
  }
  func insert(_ event:NativeSessionEvent)throws {
    let calls=event.event_kind == .assistant_end ? try SessionValidation.payload(SessionAssistantEnd.self,event).calls.count : 0
    let plans=event.event_kind == .task_transition && (try? SessionValidation.payload(SessionRequestLifecycle.self,event).plan) != nil ? 1 : 0
    try db.statement("INSERT INTO session_event_links VALUES(?,?,?,?,?,?,?)",[.text(event.agent_session_id),.text(event.event_id),.integer(Int64(event.event_sequence)),.text(event.event_kind.rawValue),event.parent_event_id.value.map(SQLValue.text) ?? .null,.integer(Int64(calls)),.integer(Int64(plans))])
    switch event.event_kind {
    case .assistant_end:
      let payload=try SessionValidation.payload(SessionAssistantEnd.self,event)
      for call in payload.calls {try db.statement("INSERT INTO session_tool_index VALUES(?,?,?,?,NULL)",[.text(event.agent_session_id),.text(call.callID),.text(event.event_id),.integer(Int64(event.event_sequence))])}
    case .tool_admitted:
      let admission=try SessionValidation.payload(SessionToolAdmission.self,event)
      var proposal:String?,existing:String?
      try db.statement("SELECT proposal_event,admission_event FROM session_tool_index WHERE session_id=? AND call_id=?",[.text(event.agent_session_id),.text(admission.callID)]) {row in
        proposal=try SQLColumn.text(row,0);if sqlite3_column_type(row,1) != SQLITE_NULL {existing=try SQLColumn.text(row,1)}
      }
      guard proposal==admission.proposalEvent,existing==nil else {throw StorageError.corruptIdentity}
      try db.statement("UPDATE session_tool_index SET admission_event=? WHERE session_id=? AND call_id=?",[.text(event.event_id),.text(event.agent_session_id),.text(admission.callID)])
    case .task_transition:
      let payload=try SessionValidation.payload(SessionRequestLifecycle.self,event)
      if let plan=payload.plan {try db.statement("INSERT INTO session_request_index VALUES(?,?,?,?,?)",[.text(event.agent_session_id),.text(plan.contextID),.text(event.event_id),.integer(Int64(event.event_sequence)),.text(event.event_id)])}
      else if let transition=payload.transition {
        var parent:String?;try db.statement("SELECT latest_event FROM session_request_index WHERE session_id=? AND context_id=?",[.text(event.agent_session_id),.text(transition.contextID)]) {parent=try SQLColumn.text($0,0)}
        guard parent==event.parent_event_id.value else {throw StorageError.corruptIdentity}
        try db.statement("UPDATE session_request_index SET latest_event=? WHERE session_id=? AND context_id=?",[.text(event.event_id),.text(event.agent_session_id),.text(transition.contextID)])
      }
    default:break
    }
  }
  func verify(_ event:NativeSessionEvent)throws {
    var matched=false
    try db.statement("SELECT event_sequence,event_kind,parent_event_id,tool_count,request_count FROM session_event_links WHERE session_id=? AND event_id=?",[.text(event.agent_session_id),.text(event.event_id)]) {row in
      let parent=sqlite3_column_type(row,2)==SQLITE_NULL ? nil : try SQLColumn.text(row,2)
      let calls=event.event_kind == .assistant_end ? try SessionValidation.payload(SessionAssistantEnd.self,event).calls.count : 0
      let plans=event.event_kind == .task_transition && (try? SessionValidation.payload(SessionRequestLifecycle.self,event).plan) != nil ? 1 : 0
      guard sqlite3_column_int64(row,0)==Int64(event.event_sequence),try SQLColumn.text(row,1)==event.event_kind.rawValue,parent==event.parent_event_id.value,
        sqlite3_column_int64(row,3)==Int64(calls),sqlite3_column_int64(row,4)==Int64(plans) else {throw StorageError.corruptIdentity};matched=true
    };guard matched else {throw StorageError.corruptIdentity}
    if event.event_kind == .assistant_end {
      let end=try SessionValidation.payload(SessionAssistantEnd.self,event)
      var calls=Set<String>()
      try db.statement("SELECT call_id,proposal_sequence FROM session_tool_index WHERE session_id=? AND proposal_event=?",[.text(event.agent_session_id),.text(event.event_id)]) {row in
        guard sqlite3_column_int64(row,1)==Int64(event.event_sequence) else {throw StorageError.corruptIdentity};calls.insert(try SQLColumn.text(row,0))
      };guard calls==Set(end.calls.map(\.callID)) else {throw StorageError.corruptIdentity}
    }
    if event.event_kind == .tool_admitted {
      let payload=try SessionValidation.payload(SessionToolAdmission.self,event);var correct=false
      try db.statement("SELECT proposal_event,admission_event FROM session_tool_index WHERE session_id=? AND call_id=?",[.text(event.agent_session_id),.text(payload.callID)]) {row in
        guard try SQLColumn.text(row,0)==payload.proposalEvent,try SQLColumn.text(row,1)==event.event_id else {throw StorageError.corruptIdentity};correct=true
      };guard correct else {throw StorageError.corruptIdentity}
    }
  }
  func children(session:String,parent:String,fetch:(String)throws->NativeSessionEvent)throws->[NativeSessionEvent] {
    var ids:[String]=[]
    try db.statement("SELECT event_id FROM session_event_links WHERE session_id=? AND parent_event_id=? AND event_kind IN ('tool_started','tool_result','task_transition') ORDER BY event_sequence LIMIT 4",[.text(session),.text(parent)]) {ids.append(try SQLColumn.text($0,0))}
    guard ids.count<=3 else {throw StorageError.corruptIdentity}
    return try ids.map {id in let event=try fetch(id);guard event.agent_session_id==session,event.parent_event_id.value==parent else {throw StorageError.corruptIdentity};return event}
  }
  func related(_ input:SessionAppendInput,fetch:(String)throws->NativeSessionEvent)throws->[NativeSessionEvent] {
    var ids=Set<String>(),parents=Set<String>()
    if let parent=input.parentEvent {ids.insert(parent);parents.insert(parent)}
    switch input.kind {
    case .assistant_end:
      let end=try JSONDecoder().decode(SessionAssistantEnd.self,from:input.payload)
      for call in end.calls {
        try db.statement("SELECT proposal_event FROM session_tool_index WHERE session_id=? AND call_id=?",[.text(input.session),.text(call.callID)]) {ids.insert(try SQLColumn.text($0,0))}
      }
    case .tool_admitted:
      let admission=try JSONDecoder().decode(SessionToolAdmission.self,from:input.payload);ids.insert(admission.proposalEvent)
      try db.statement("SELECT admission_event FROM session_tool_index WHERE session_id=? AND call_id=? AND admission_event IS NOT NULL",[.text(input.session),.text(admission.callID)]) {ids.insert(try SQLColumn.text($0,0))}
    case .task_transition:
      let payload=try JSONDecoder().decode(SessionRequestLifecycle.self,from:input.payload)
      if let plan=payload.plan {try db.statement("SELECT plan_event FROM session_request_index WHERE session_id=? AND context_id=?",[.text(input.session),.text(plan.contextID)]) {ids.insert(try SQLColumn.text($0,0))}}
    default:break
    }
    var result:[NativeSessionEvent]=[]
    for id in ids {result.append(try fetch(id))}
    for parent in parents {result += try children(session:input.session,parent:parent,fetch:fetch);result += try readonlyResults(session:input.session,parent:parent,fetch:fetch)}
    return result.sorted{$0.event_sequence<$1.event_sequence}
  }
  func readonlyResults(session:String,parent:String,fetch:(String)throws->NativeSessionEvent)throws->[NativeSessionEvent] {
    var ids:[String]=[]
    try db.statement("SELECT event_id FROM session_event_links WHERE session_id=? AND parent_event_id=? AND event_kind='recovery' ORDER BY event_sequence LIMIT 33",[.text(session),.text(parent)]) {ids.append(try SQLColumn.text($0,0))}
    guard ids.count<=32 else {throw StorageError.corruptIdentity}
    return try ids.map {id in let event=try fetch(id);guard event.agent_session_id==session,event.parent_event_id.value==parent,event.event_kind == .recovery else {throw StorageError.corruptIdentity};return event}
  }
  struct ToolRow {let call:String,proposal:String,sequence:UInt64,admission:String?}
  struct RequestRow {let context:String,plan:String,sequence:UInt64,latest:String}
  func tools(session:String,upper:UInt64,after:UInt64,afterID:String,limit:Int)throws->[ToolRow] {
    var rows:[ToolRow]=[]
    try db.statement("SELECT call_id,proposal_event,proposal_sequence,admission_event FROM session_tool_index WHERE session_id=? AND proposal_sequence<=? AND (proposal_sequence>? OR (proposal_sequence=? AND call_id>?)) ORDER BY proposal_sequence,call_id LIMIT ?",[.text(session),.integer(Int64(upper)),.integer(Int64(after)),.integer(Int64(after)),.text(afterID),.integer(Int64(limit))]) {row in
      let sequence=sqlite3_column_int64(row,2);guard sequence>0,sequence<=Int64(HostSerial.maximum) else {throw StorageError.corruptIdentity}
      rows.append(.init(call:try SQLColumn.text(row,0),proposal:try SQLColumn.text(row,1),sequence:UInt64(sequence),admission:sqlite3_column_type(row,3)==SQLITE_NULL ? nil : try SQLColumn.text(row,3)))
    };return rows
  }
  func requests(session:String,upper:UInt64,after:UInt64,limit:Int)throws->[RequestRow] {
    var rows:[RequestRow]=[]
    try db.statement("SELECT context_id,plan_event,plan_sequence,latest_event FROM session_request_index WHERE session_id=? AND plan_sequence<=? AND plan_sequence>? ORDER BY plan_sequence LIMIT ?",[.text(session),.integer(Int64(upper)),.integer(Int64(after)),.integer(Int64(limit))]) {row in
      let sequence=sqlite3_column_int64(row,2);guard sequence>0,sequence<=Int64(HostSerial.maximum) else {throw StorageError.corruptIdentity}
      rows.append(.init(context:try SQLColumn.text(row,0),plan:try SQLColumn.text(row,1),sequence:UInt64(sequence),latest:try SQLColumn.text(row,3)))
    };return rows
  }
}
