import React, { memo, useCallback, useMemo, useState } from 'react';
import FocusTrap from 'focus-trap-react';
import {
  Box,
  Button,
  Chip,
  Dialog,
  Header,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Text,
  color,
  config,
  toRem,
} from 'folds';
import { useTranslation } from 'react-i18next';
import { HermesAgentPayload, HermesCodeBlock, hermesPayloadToMarkdown } from '../../utils/hermes';
import { copyToClipboard } from '../../utils/dom';
import { sendPlatformAgentAction } from '../../platform';
import { openExternalUrl } from '../../utils/appLinks';
import { NativeCodeBlock } from '../../features/room/nativeTimelineFormattedBody';

type CodeSectionProps = {
  title: string;
  blocks: HermesCodeBlock[];
  copyLabel: string;
};

type ActionEntry = HermesAgentPayload['actions'][number];

function HermesCodePre({ block }: { block: HermesCodeBlock }) {
  const languageClass = block.language ? `language-${block.language}` : undefined;
  return <NativeCodeBlock code={block.code} languageClass={languageClass} />;
}

const CodeSection = memo(({ title, blocks, copyLabel }: CodeSectionProps) => {
  const [openBlocks, setOpenBlocks] = useState<Set<string>>(() => new Set());

  if (blocks.length === 0) return null;

  return (
    <Box direction="Column" gap="200">
      <Text size="L400">
        {title} ({blocks.length})
      </Text>
      {blocks.map((block) => (
        <Box
          key={block.id}
          as="details"
          direction="Column"
          gap="200"
          open={openBlocks.has(block.id)}
          onToggle={(evt: React.SyntheticEvent<HTMLDetailsElement>) => {
            const { open } = evt.currentTarget;
            setOpenBlocks((current) => {
              const next = new Set(current);
              if (open) next.add(block.id);
              else next.delete(block.id);
              return next;
            });
          }}
          style={{
            contain: 'layout paint style',
            border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
            borderRadius: config.radii.R300,
            padding: config.space.S300,
          }}
        >
          <Box as="summary" alignItems="Center" gap="200" style={{ cursor: 'pointer' }}>
            <Text as="span" size="T300" truncate>
              {block.title ?? block.language ?? title}
            </Text>
            {block.language && (
              <Chip size="400" radii="Pill" outlined>
                <Text size="L400">{block.language}</Text>
              </Chip>
            )}
          </Box>
          <Box justifyContent="End">
            <Button
              size="300"
              variant="Secondary"
              fill="None"
              before={<Icon size="100" src={Icons.Link} />}
              onClick={() => copyToClipboard(block.code)}
            >
              <Text size="B300">{copyLabel}</Text>
            </Button>
          </Box>
          {openBlocks.has(block.id) && <HermesCodePre block={block} />}
        </Box>
      ))}
    </Box>
  );
});

type HermesAgentCardProps = {
  payload: HermesAgentPayload;
};

export const HermesAgentCard = memo(({ payload }: HermesAgentCardProps) => {
  const { t } = useTranslation();
  const markdown = hermesPayloadToMarkdown(payload);
  const [busyActionId, setBusyActionId] = useState<string>();
  const [actionErrorId, setActionErrorId] = useState<string>();
  const [pendingUrlAction, setPendingUrlAction] = useState<ActionEntry>();

  const pendingActionTitle = pendingUrlAction?.title ?? '';
  const pendingActionUrl = pendingUrlAction?.url ?? '';

  const copyCodeLabel = t('modernization.hermes.copy_code', 'Copy');
  const copyMarkdownLabel = t('modernization.hermes.copy_markdown', 'Copy as Markdown');
  const copyJsonLabel = t('modernization.hermes.copy_json', 'Copy JSON');
  const copyLinksLabel = t('modernization.hermes.copy_links', 'Copy Links');
  const openLabel = t('modernization.hermes.open', 'Open');
  const logsTitle = t('modernization.hermes.logs_title', 'Logs');
  const codeTitle = t('modernization.hermes.code_title', 'Code');
  const diffsTitle = t('modernization.hermes.diffs_title', 'Diffs');
  const actionsTitle = t('modernization.hermes.actions_title', 'Actions');
  const artifactsTitle = t('modernization.hermes.artifacts_title', 'Artifacts');

  const handleCopyMarkdown = useCallback(() => copyToClipboard(markdown), [markdown]);
  const handleCopyJson = useCallback(
    () => copyToClipboard(JSON.stringify(payload, null, 2)),
    [payload]
  );
  const handleCopyArtifactLinks = useCallback(
    () =>
      copyToClipboard(
        payload.artifacts
          .filter((artifact) => artifact.url)
          .map((artifact) => `${artifact.title}: ${artifact.url}`)
          .join('\n')
      ),
    [payload.artifacts]
  );

  const handleOpenUrl = useCallback(
    async (action: ActionEntry) => {
      setBusyActionId(action.id);
      setActionErrorId(undefined);
      try {
        const handled = await sendPlatformAgentAction({
          ...action,
          markdown,
        });
        if (!handled) {
          copyToClipboard(action.prompt ?? action.title);
          setActionErrorId(undefined);
        }
      } catch {
        setActionErrorId(action.id);
      } finally {
        setBusyActionId(undefined);
        setPendingUrlAction(undefined);
      }
    },
    [markdown]
  );

  const handleActionClick = useCallback(
    async (action: ActionEntry) => {
      if (action.url) {
        setPendingUrlAction(action);
        return;
      }
      await handleOpenUrl(action);
    },
    [handleOpenUrl]
  );

  const handleActionOpenConfirmed = useCallback(() => {
    if (!pendingUrlAction?.url) return;
    void openExternalUrl(pendingUrlAction.url);
    setPendingUrlAction(undefined);
  }, [pendingUrlAction]);

  const handleActionOpenCancel = useCallback(() => {
    setPendingUrlAction(undefined);
  }, []);

  const openArtifact = useCallback((title: string, url: string) => {
    setPendingUrlAction({
      id: `artifact-${title}`,
      title,
      url,
    });
  }, []);

  const actionRow = useMemo(
    () =>
      payload.actions.map((action) => (
        <Button
          key={action.id}
          disabled={busyActionId === action.id}
          size="300"
          variant={action.url ? 'Primary' : 'Secondary'}
          fill="Soft"
          before={action.url ? <Icon size="100" src={Icons.External} /> : undefined}
          onClick={() => handleActionClick(action)}
        >
          <Text size="B300">{action.title}</Text>
        </Button>
      )),
    [busyActionId, handleActionClick, payload.actions]
  );

  const artifactRows = useMemo(
    () =>
      payload.artifacts.map((artifact) => (
        <Box
          key={`${artifact.title}-${artifact.url ?? artifact.type ?? ''}`}
          alignItems="Center"
          justifyContent="SpaceBetween"
          gap="300"
          style={{
            border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
            borderRadius: config.radii.R300,
            padding: config.space.S300,
          }}
        >
          <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0 }}>
            <Text size="T300" truncate>
              {artifact.title}
            </Text>
            {(artifact.type || artifact.summary) && (
              <Text size="T200" priority="300" truncate>
                {[artifact.type, artifact.summary].filter(Boolean).join(' - ')}
              </Text>
            )}
          </Box>
          {artifact.url && (
            <Button
              disabled={!!busyActionId}
              size="300"
              onClick={() => {
                if (artifact.url) openArtifact(artifact.title, artifact.url);
              }}
            >
              <Text size="B300">{openLabel}</Text>
              <Icon size="100" src={Icons.External} />
            </Button>
          )}
        </Box>
      )),
    [busyActionId, openArtifact, payload.artifacts, openLabel]
  );

  return (
    <Box
      direction="Column"
      gap="300"
      style={{
        maxWidth: toRem(760),
        border: `${config.borderWidth.B300} solid ${color.Surface.ContainerLine}`,
        borderRadius: config.radii.R400,
        padding: config.space.S400,
      }}
    >
      {pendingUrlAction ? (
        <Overlay open backdrop={<OverlayBackdrop />}>
          <OverlayCenter>
            <FocusTrap
              focusTrapOptions={{
                initialFocus: false,
                clickOutsideDeactivates: true,
                escapeDeactivates: true,
              }}
            >
              <Dialog variant="Primary">
                <Header
                  style={{
                    padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                    borderBottomWidth: config.borderWidth.B300,
                  }}
                  variant="Surface"
                  size="300"
                >
                  <Box grow="Yes">
                    <Text size="H4">{t('modernization.hermes.confirm_action', 'Open link')}</Text>
                  </Box>
                  <IconButton size="300" onClick={handleActionOpenCancel} radii="300">
                    <Icon src={Icons.Cross} />
                  </IconButton>
                </Header>
                <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
                  <Text priority="400">
                    {t('modernization.hermes.confirm_action_description', {
                      title: pendingActionTitle,
                      defaultValue: `Open "${pendingActionTitle}" in an external browser?`,
                    })}
                  </Text>
                  <Text size="T200" priority="300" style={{ wordBreak: 'break-word' }}>
                    {pendingActionUrl}
                  </Text>
                  <Box direction="Row" gap="200" justifyContent="End">
                    <Button variant="Secondary" size="300" onClick={handleActionOpenCancel}>
                      <Text size="B300">
                        {t('modernization.hermes.confirm_action_cancel', 'Cancel')}
                      </Text>
                    </Button>
                    <Button variant="Primary" size="300" onClick={handleActionOpenConfirmed}>
                      <Text size="B300">
                        {t('modernization.hermes.confirm_action_open', 'Open')}
                      </Text>
                    </Button>
                  </Box>
                </Box>
              </Dialog>
            </FocusTrap>
          </OverlayCenter>
        </Overlay>
      ) : null}

      <Box justifyContent="SpaceBetween" alignItems="Start" gap="300">
        <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0 }}>
          <Text size="H5" truncate>
            {payload.title}
          </Text>
          {payload.summary && <Text priority="300">{payload.summary}</Text>}
        </Box>
        {payload.status && (
          <Chip variant="Secondary" radii="Pill" outlined>
            <Text size="L400">{payload.status}</Text>
          </Chip>
        )}
      </Box>

      <Box justifyContent="End" gap="100" wrap="Wrap">
        <Button
          size="300"
          variant="Secondary"
          fill="None"
          before={<Icon size="100" src={Icons.Link} />}
          onClick={handleCopyMarkdown}
        >
          <Text size="B300">{copyMarkdownLabel}</Text>
        </Button>
        <Button
          size="300"
          variant="Secondary"
          fill="None"
          before={<Icon size="100" src={Icons.Code} />}
          onClick={handleCopyJson}
        >
          <Text size="B300">{copyJsonLabel}</Text>
        </Button>
        {payload.artifacts.some((artifact) => artifact.url) && (
          <Button
            size="300"
            variant="Secondary"
            fill="None"
            before={<Icon size="100" src={Icons.Link} />}
            onClick={handleCopyArtifactLinks}
          >
            <Text size="B300">{copyLinksLabel}</Text>
          </Button>
        )}
      </Box>

      {payload.actions.length > 0 ? (
        <Box direction="Column" gap="200">
          <Text size="L400">{`${actionsTitle} (${payload.actions.length})`}</Text>
          {actionErrorId && (
            <Text size="T200" style={{ color: color.Critical.Main }}>
              {t('modernization.hermes.action_failed', 'Action failed')}
            </Text>
          )}
          <Box gap="100" wrap="Wrap">
            {actionRow}
          </Box>
        </Box>
      ) : null}

      {payload.artifacts.length > 0 ? (
        <Box direction="Column" gap="200">
          <Text size="L400">{`${artifactsTitle} (${payload.artifacts.length})`}</Text>
          {artifactRows}
        </Box>
      ) : null}

      <CodeSection title={logsTitle} blocks={payload.logs} copyLabel={copyCodeLabel} />
      <CodeSection title={codeTitle} blocks={payload.code} copyLabel={copyCodeLabel} />
      <CodeSection title={diffsTitle} blocks={payload.diffs} copyLabel={copyCodeLabel} />
    </Box>
  );
});
