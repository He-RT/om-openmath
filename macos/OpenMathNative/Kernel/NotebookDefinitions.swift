import Foundation

struct NotebookDefinition:Decodable,Sendable,Identifiable {
  let name:String,value:String?,cell_id:String,stale:Bool,error:Bool
  var id:String {name}
}
struct NotebookDefinitionsReply:Decodable,Sendable {
  let protocol_version:UInt32,document_id:String,generation:HostSerial,base_hash:String
  let checkpoint_id:String?,kernel_state_revision:HostSerial
  let items:[NotebookDefinition]
}
