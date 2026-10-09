import Foundation
func status(_ id:String,_ phase:HostOperationStatusPhase = .completed)->HostOperationStatus {
  .init(operation_ref:id,phase:phase,result:.init(.object(["value":.string("4")])),error_code:.init(nil))
}
func snapshot(_ sequence:UInt64,_ statuses:[HostOperationStatus]=[],document:DocumentBinding?=nil)->HostSnapshot {
  .init(protocol_version:1,runtime_instance_id:"router-test",host_phase:.ready,
    rust_event_sequence:try! .init(sequence),document_binding:.init(document),document_storage_ready:false,
    operation_history_complete:false,operations:statuses,unavailable_operation_refs:[])
}
func event(_ sequence:UInt64,_ id:String,runtime:String="router-test",document:DocumentBinding?=nil)->NativeDecodedEvent {
  .init(envelope:.init(protocol_version:1,runtime_instance_id:runtime,rust_event_sequence:try! .init(sequence),
    request_ref:.init(id),operation_ref:.init(id),document_binding:.init(document),event_kind:.operation_finished,
    payload:.object([:])),operation:status(id))
}
func check(_ condition:@autoclosure () throws ->Bool) throws { let value=try condition(); precondition(value) }
var router=AppEventRouter(runtime:"router-test")
try check( router.install(snapshot(1)))
let normal=NativeDecodedBatch(needsResync:false,events:[event(2,"one")])
try check( router.apply(normal).count==1)
let ui=router.projection.uiSequence
try check( router.apply(normal).isEmpty)
precondition(router.projection.uiSequence==ui)
try check( router.apply(.init(needsResync:false,events:[event(3,"foreign",runtime:"old")])).isEmpty)
precondition(router.projection.rustSequence==2)
try check( router.apply(.init(needsResync:false,events:[event(4,"gap")])).isEmpty)
precondition(router.projection.needsResync)
precondition(router.projection.operations["gap"]==nil)
try check( router.install(snapshot(4,[status("one"),status("gap")])))
precondition(!router.projection.needsResync)
try check( router.apply(.init(needsResync:false,events:[event(3,"delayed")])).isEmpty)
let document=DocumentBinding(document_id:"old-doc",generation:try! .init(1),document_revision:try! .init(1),execution_epoch:try! .init(1))
try check( router.apply(.init(needsResync:false,events:[event(5,"old-doc-job",document:document)])).isEmpty)
precondition(router.projection.rustSequence==5)
precondition(router.projection.operations["old-doc-job"]==nil)
router.registerLocalProducer("storage",identity:"storage-1",generation:1)
router.registerLocalProducer("pi",identity:"connection-1",generation:4)
try check( router.applyLocal("storage",identity:"storage-1",generation:1,sequence:1))
try check( router.applyLocal("pi",identity:"connection-1",generation:4,sequence:1))
precondition(router.projection.rustSequence==5)
try check( !router.applyLocal("pi",identity:"old-connection",generation:4,sequence:2))
try check( !router.applyLocal("pi",identity:"connection-1",generation:3,sequence:2))
try check( !router.applyLocal("pi",identity:"connection-1",generation:4,sequence:1))
try check( !router.applyLocal("pi",identity:"connection-1",generation:4,sequence:3))
try check( router.apply(.init(needsResync:true,events:[event(6,"dropped-baseline")])).isEmpty)
precondition(router.projection.needsResync)
precondition(router.projection.operations["dropped-baseline"]==nil)
try check( router.install(snapshot(6,[status("dropped-baseline")])))
print("Pure reducer: duplicates, gap fencing/readback, stale runtime/document, independent producer counters and explicit backpressure passed")
var fenced=AppEventRouter(runtime:"router-test")
try check(fenced.install(snapshot(6)))
_ = try fenced.apply(.init(needsResync:true,lastRustSequence:8,events:[]))
try check(!fenced.install(snapshot(7)))
precondition(fenced.projection.needsResync)
try check(fenced.install(snapshot(8)))
print("Empty overflow batch carries its actual fence; an older in-flight snapshot cannot clear recovery")

try fenced.confirmClosed()
let closedSequence=fenced.projection.uiSequence
try check(fenced.apply(.init(needsResync:false,lastRustSequence:9,events:[event(9,"late-after-close")])).isEmpty)
precondition(fenced.projection.uiSequence==closedSequence)
print("A consumed host cannot be reopened by late callbacks")
