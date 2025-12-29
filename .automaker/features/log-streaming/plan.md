# Log Streaming Feature Plan

## Goal
Wire package operations to emit real-time logs to the Task Hub via `AppendTaskLog`.

## Status: IN_PROGRESS

---

## Steps

### Phase 1: Task ID Tracking
- [x] 1.1 Add `task_id: usize` field to `BeginTask` and `BeginBatchTask` inputs
- [x] 1.2 Add `next_task_id: usize` counter to AppModel  
- [x] 1.3 Update `OperationStarted` message to include task_id
- [x] 1.4 Generate task_id in `ExecutePackageAction` before async spawn
- [x] 1.5 Pass task_id through to OperationStarted/OperationCompleted/OperationFailed

### Phase 2: Message Flow Update
- [x] 2.1 Update `OperationStarted` handler to accept and use task_id
- [x] 2.2 Update `OperationCompleted` handler to use FinishTask with task_id
- [x] 2.3 Update `OperationFailed` handler to use FinishTask with task_id
- [x] 2.4 Cleanup ops (CleanSource, RemoveOrphans) now use OperationCompleted/Failed
- [x] 2.5 DowngradePackage now uses task_id tracking

### Phase 3: Log Emission Helper
- [x] 3.1 Create `AppMsg::AppendLog { task_id, line }` message
- [x] 3.2 Handle AppendLog in update() to forward to TaskHub
- [ ] 3.3 Create helper function for streaming command output

### Phase 4: Backend Integration
- [ ] 4.1 Update one backend (apt) to emit logs during operations
- [ ] 4.2 Test log streaming end-to-end
- [ ] 4.3 Extend to other backends as needed

### Phase 5: Polish
- [ ] 5.1 Strip ANSI escape codes from log lines
- [x] 5.2 Run clippy and fmt
- [ ] 5.3 Commit and push

---

## Current Step: 3.3 - Create streaming helper

## Architecture Notes
Flow: ExecutePackageAction → spawn async → OperationStarted(task_id) → BeginTask
                                         → emit logs → AppendLog(task_id)
                                         → OperationCompleted(task_id) → FinishTask

## Completed This Session
- Fixed all compilation errors from task_id additions
- Updated CleanSource and RemoveOrphans cleanup operations to use unified OperationCompleted/Failed pattern
- Updated DowngradePackage to use task_id tracking
- All quality gates pass (clippy, fmt)
