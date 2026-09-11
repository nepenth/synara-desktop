import SwiftUI

/// One owner for presentation and acknowledgement of native verification flows.
@MainActor
final class CryptoVerificationPresentation: ObservableObject {
    @Published private(set) var snapshot: CryptoVerificationSnapshot?
    @Published private(set) var isDismissing = false
    @Published var error: String?
    private var acknowledged: Set<CryptoVerificationSnapshot.ID> = []
    private var dismissingID: CryptoVerificationSnapshot.ID?
    private var generation: UInt64?
    private var lifecycle = 0

    func receive(_ update: CryptoVerificationSnapshot?) {
        if let update {
            if let generation, update.id.sessionGeneration < generation { return }
            if generation != update.id.sessionGeneration {
                generation = update.id.sessionGeneration
                acknowledged.removeAll()
            }
            guard !acknowledged.contains(update.id), update.id != dismissingID else { return }
        } else if isDismissing {
            return
        }
        snapshot = update
    }

    func dismiss(using crypto: CryptoStatusServicing) async {
        guard let current = snapshot, current.state.isTerminal, !isDismissing else { return }
        let expectedLifecycle = lifecycle
        isDismissing = true
        dismissingID = current.id
        error = nil
        defer {
            if lifecycle == expectedLifecycle {
                isDismissing = false
                dismissingID = nil
            }
        }
        let result = await crypto.dismissVerification(flowID: current.id.flowID)
        guard lifecycle == expectedLifecycle else { return }
        switch result {
        case .completed:
            acknowledged.insert(current.id)
            if snapshot?.id == current.id { snapshot = nil }
        case .failed(let message), .unavailable(let message):
            if snapshot?.id == current.id { error = message }
        }
    }

    func reset() {
        lifecycle += 1
        isDismissing = false
        dismissingID = nil
        snapshot = nil
        acknowledged.removeAll()
        generation = nil
        error = nil
    }
}
