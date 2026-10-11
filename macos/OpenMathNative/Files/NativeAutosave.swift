import Foundation

/// Idle and maximum-delay clocks coalesce edits. A failed/unknown save pauses the producer;
/// it never retries a new operation silently or chooses a target for an untitled document.
@MainActor final class NativeAutosave {
  private let idleDelay:Duration
  private let maximumDelay:Duration
  private let save:@MainActor ()async throws->Bool
  private var idle:Task<Void,Never>?
  private var maximum:Task<Void,Never>?
  private var running:Task<Void,Never>?
  private var dirty=false
  private var paused=false
  private var closed=false
  private(set) var failure:(any Error)?
  init(idleDelay:Duration = .seconds(2),maximumDelay:Duration = .seconds(10),save:@escaping @MainActor ()async throws->Bool) {
    self.idleDelay=idleDelay;self.maximumDelay=maximumDelay;self.save=save
  }
  func edited() {
    guard !closed else {return};dirty=true
    guard !paused else {return}
    schedule()
  }
  private func schedule() {
    idle?.cancel()
    idle=Task { [weak self,idleDelay] in
      do {try await Task.sleep(for:idleDelay)} catch {return}
      self?.fire()
    }
    if maximum==nil {maximum=Task { [weak self,maximumDelay] in
      do {try await Task.sleep(for:maximumDelay)} catch {return}
      self?.fire()
    }}
  }
  private func fire() {
    guard !closed,!paused,dirty else {return}
    idle?.cancel();idle=nil;maximum?.cancel();maximum=nil
    guard running==nil else {return}
    dirty=false
    running=Task { [weak self] in
      guard let self else {return}
      do {
        let clean=try await self.save()
        self.dirty=self.dirty || !clean
      } catch {self.failure=error;self.paused=true;self.dirty=true}
      self.running=nil
      if !self.closed,!self.paused,self.dirty {self.schedule()}
    }
  }
  /// A deliberate successful save/target choice re-enables automation. Failure does not.
  func manualSaved(clean:Bool) {guard !closed else {return};paused=false;failure=nil;dirty = !clean;if dirty {schedule()} else {idle?.cancel();idle=nil;maximum?.cancel();maximum=nil}}
  func suspend() {paused=true;idle?.cancel();idle=nil;maximum?.cancel();maximum=nil}
  func finish() async {closed=true;suspend();await running?.value;running=nil}
}
