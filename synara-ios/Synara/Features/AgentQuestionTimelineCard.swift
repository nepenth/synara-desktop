import SwiftUI

/// A Hermes clarifying question in the timeline. Mirrors the approval card:
/// the question, one button per choice (the recommended one prominent),
/// toggles plus Send for multi-select, an "Other…" field and Skip. A settled
/// card shows one line instead of controls. Core chooses the answer route.
struct AgentQuestionTimelineCard: View {
    let question: SynaraAgentQuestion
    let eventID: String
    /// This account's latest reply to the card in the loaded window.
    let ownReplyBody: String?
    /// Sends an answer through Core; true when it was sent. Nil where the
    /// card cannot be answered from this surface.
    let onAnswer: ((SynaraAgentQuestionAnswer) async -> Bool)?

    @State private var pendingAnswer: SynaraAgentQuestionAnswer?
    @State private var localAnswer: SynaraAgentQuestionAnswer?
    @State private var selection: [UInt32] = []
    @State private var otherText = ""

    private var settlement: SynaraAgentQuestionSettlement {
        SynaraAgentQuestionPresentation.settle(
            question,
            now: Date(),
            ownReplyBody: ownReplyBody,
            localAnswer: localAnswer
        )
    }

    private var isInteractive: Bool {
        settlement == .pending && question.canAnswer && onAnswer != nil
    }

    private var isBusy: Bool { pendingAnswer != nil }

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                Image(systemName: "questionmark.bubble.fill")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(SynaraColor.accentText)
                    .accessibilityHidden(true)
                Text(question.question)
                    .font(SynaraTypography.emphasis)
                    .foregroundStyle(SynaraColor.primaryText)
                    .fixedSize(horizontal: false, vertical: true)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("AgentQuestionText-\(eventID)")
            }

            if let title = settlement.title {
                settledLine(title)
            } else if isInteractive {
                if question.choices.isEmpty == false {
                    choiceControls
                }
                if question.allowOther {
                    otherField
                }
                if question.allowSkip {
                    HStack {
                        Spacer(minLength: 0)
                        Button {
                            submit(.skip)
                        } label: {
                            busyLabel(String(localized: "Skip"), answer: .skip)
                        }
                        .buttonStyle(.bordered)
                        .tint(SynaraColor.secondaryText)
                        .disabled(isBusy)
                        .accessibilityIdentifier("AgentQuestionSkip-\(eventID)")
                    }
                }
            } else {
                readOnlyChoices
                if question.canAnswer == false {
                    Text(String(localized: "This question is for someone else."))
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityIdentifier("AgentQuestionReadOnly-\(eventID)")
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(String(localized: "Question from the agent"))
        .accessibilityIdentifier("AgentQuestionCard-\(eventID)")
    }

    // MARK: Controls

    @ViewBuilder
    private var choiceControls: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            ForEach(question.choices) { choice in
                if question.multiSelect {
                    multiSelectToggle(choice)
                } else {
                    singleChoiceButton(choice)
                }
            }
            if question.multiSelect {
                Button {
                    submit(.choices(selection))
                } label: {
                    busyLabel(String(localized: "Send"), answer: .choices(selection), systemImage: "paperplane.fill")
                        .frame(maxWidth: .infinity, minHeight: SynaraHitTarget.minimum)
                }
                .buttonStyle(.borderedProminent)
                .tint(SynaraColor.accentFill)
                .disabled(isBusy || selection.isEmpty)
                .accessibilityLabel(String(localized: "Send selected answers"))
                .accessibilityIdentifier("AgentQuestionSend-\(eventID)")
            }
        }
    }

    @ViewBuilder
    private func singleChoiceButton(_ choice: SynaraAgentQuestion.Choice) -> some View {
        let button = Button {
            submit(.choice(choice.index))
        } label: {
            HStack(spacing: SynaraSpacing.small) {
                if pendingAnswer == .choice(choice.index) {
                    ProgressView().controlSize(.small)
                }
                Text(choice.label)
                    .font(.subheadline.weight(.semibold))
                    .multilineTextAlignment(.leading)
                Spacer(minLength: 0)
                if choice.recommended {
                    Label(String(localized: "Recommended"), systemImage: "star.fill")
                        .labelStyle(.iconOnly)
                        .accessibilityHidden(true)
                }
            }
            .frame(maxWidth: .infinity, minHeight: SynaraHitTarget.minimum)
            .contentShape(Rectangle())
            .foregroundStyle(choice.recommended ? Color.white : SynaraColor.primaryText)
        }
        .controlSize(.large)
        .disabled(isBusy)
        .accessibilityLabel(choiceAccessibilityLabel(choice))
        .accessibilityIdentifier("AgentQuestionChoice-\(choice.index)-\(eventID)")

        // The recommended choice is the expected path; the rest stay quieter.
        if choice.recommended {
            button.buttonStyle(.borderedProminent).tint(SynaraColor.accentFill)
        } else {
            button.buttonStyle(.bordered).tint(SynaraColor.secondaryText)
        }
    }

    private func multiSelectToggle(_ choice: SynaraAgentQuestion.Choice) -> some View {
        let selected = selection.contains(choice.index)
        return Button {
            selection = SynaraAgentQuestionPresentation.toggle(choice.index, in: selection)
        } label: {
            HStack(spacing: SynaraSpacing.small) {
                Image(systemName: selected ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(selected ? SynaraColor.accentText : SynaraColor.secondaryText)
                    .accessibilityHidden(true)
                Text(choice.label)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                    .multilineTextAlignment(.leading)
                Spacer(minLength: 0)
                if choice.recommended {
                    Text(String(localized: "Recommended"))
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityHidden(true)
                }
            }
            .frame(maxWidth: .infinity, minHeight: SynaraHitTarget.minimum)
            .contentShape(Rectangle())
        }
        .buttonStyle(.bordered)
        .tint(selected ? SynaraColor.accentFill : SynaraColor.secondaryText)
        .disabled(isBusy)
        .accessibilityLabel(choiceAccessibilityLabel(choice))
        .accessibilityAddTraits(selected ? .isSelected : [])
        .accessibilityIdentifier("AgentQuestionChoice-\(choice.index)-\(eventID)")
    }

    private var otherField: some View {
        HStack(spacing: SynaraSpacing.small) {
            // Single line so Return sends the answer.
            TextField(String(localized: "Other…"), text: $otherText)
                .textFieldStyle(.roundedBorder)
                .submitLabel(.send)
                .onSubmit(sendOther)
                .disabled(isBusy)
                .accessibilityLabel(String(localized: "Your own answer"))
                .accessibilityIdentifier("AgentQuestionOther-\(eventID)")
            Button(action: sendOther) {
                if case .other = pendingAnswer {
                    ProgressView().controlSize(.small)
                } else {
                    Image(systemName: "paperplane.fill")
                }
            }
            .buttonStyle(.bordered)
            .frame(minWidth: SynaraHitTarget.minimum, minHeight: SynaraHitTarget.minimum)
            .disabled(isBusy || trimmedOther.isEmpty)
            .accessibilityLabel(String(localized: "Send your own answer"))
            .accessibilityIdentifier("AgentQuestionOtherSend-\(eventID)")
        }
    }

    @ViewBuilder
    private var readOnlyChoices: some View {
        if question.choices.isEmpty == false {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                ForEach(question.choices) { choice in
                    Text(verbatim: "\(choice.index). \(choice.label)")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .accessibilityLabel(choiceAccessibilityLabel(choice))
                }
            }
        }
    }

    private func settledLine(_ title: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
            Image(systemName: settledSystemImage)
                .font(SynaraTypography.emphasis)
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityHidden(true)
            Text(title)
                .font(SynaraTypography.emphasis)
                .foregroundStyle(SynaraColor.primaryText)
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 0)
        }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("AgentQuestionSettled-\(eventID)")
    }

    private var settledSystemImage: String {
        switch settlement {
        case .answered: return "checkmark.circle.fill"
        case .skipped: return "forward.end.fill"
        case .expired: return "clock.badge.xmark"
        case .cancelled: return "xmark.circle"
        case .pending: return "questionmark.circle"
        }
    }

    @ViewBuilder
    private func busyLabel(
        _ title: String,
        answer: SynaraAgentQuestionAnswer,
        systemImage: String? = nil
    ) -> some View {
        HStack(spacing: SynaraSpacing.small) {
            if pendingAnswer == answer {
                ProgressView().controlSize(.small)
            } else if let systemImage {
                Image(systemName: systemImage).accessibilityHidden(true)
            }
            Text(title).font(.subheadline.weight(.semibold))
        }
    }

    // MARK: Actions

    private var trimmedOther: String {
        otherText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func choiceAccessibilityLabel(_ choice: SynaraAgentQuestion.Choice) -> String {
        choice.recommended
            ? String(localized: "\(choice.label), recommended")
            : choice.label
    }

    private func sendOther() {
        let text = trimmedOther
        guard text.isEmpty == false else { return }
        submit(.other(text))
    }

    /// No success alert: on success the card turns into its settled line.
    /// The caller reports a failure; the controls stay available to retry.
    private func submit(_ answer: SynaraAgentQuestionAnswer) {
        guard let onAnswer, pendingAnswer == nil, localAnswer == nil else { return }
        pendingAnswer = answer
        Task { @MainActor in
            let sent = await onAnswer(answer)
            pendingAnswer = nil
            if sent {
                localAnswer = answer
            }
        }
    }
}
