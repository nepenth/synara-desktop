import React, { useMemo, useRef, useState } from 'react';
import { Box, Button, Chip, Icon, IconButton, Icons, Scroll, Text, config, toRem } from 'folds';
import { useTranslation } from 'react-i18next';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useAtomValue } from 'jotai';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { SynaraLaterItem } from '../../../../types/matrix/accountData';
import { getSortedLaterItems } from '../../../utils/later';
import { laterContentAtom } from '../../../state/laterList';
import {
  clearCompletedLaterWithNativeOwner,
  completeLaterWithNativeOwner,
  snoozeLaterWithNativeOwner,
} from '../../../features/room/nativeLaterOwner';
import { Page, PageContent, PageContentCenter, PageHeader } from '../../../components/page';
import { ScreenSize, useScreenSizeContext } from '../../../hooks/useScreenSize';
import { BackRouteHandler } from '../../../components/BackRouteHandler';
import { useRoomNavigate } from '../../../hooks/useRoomNavigate';
import { SequenceCard } from '../../../components/sequence-card';
import { getMemberDisplayName, getThreadRootEventId } from '../../../utils/room';
import { getMxIdLocalPart } from '../../../utils/matrix';
import { VirtualTile } from '../../../components/virtualizer';

const formatDue = (dueTs?: number, now = Date.now()): string | undefined => {
  if (!dueTs) return undefined;
  const due = new Date(dueTs);
  if (dueTs <= now) return 'Due now';
  const today = new Date(now);
  const tomorrow = new Date(now);
  tomorrow.setDate(today.getDate() + 1);

  if (due.toDateString() === today.toDateString()) {
    return `Today ${due.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`;
  }
  if (due.toDateString() === tomorrow.toDateString()) {
    return `Tomorrow ${due.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`;
  }
  return due.toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
};

const toDateTimeLocal = (ts?: number): string => {
  if (!ts) return '';
  const date = new Date(ts);
  const offsetDate = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return offsetDate.toISOString().slice(0, 16);
};

const fromDateTimeLocal = (value: string): number | undefined => {
  const ts = new Date(value).getTime();
  return Number.isFinite(ts) ? ts : undefined;
};

const getEventPreview = (
  eventType: string | undefined,
  content: { body?: unknown; msgtype?: unknown } | undefined,
  fallback: string
): string => {
  if (eventType === 'm.room.encrypted') return 'Encrypted message';
  if (typeof content?.body === 'string') return content.body;
  if (typeof content?.msgtype === 'string') return content.msgtype;
  return fallback;
};

type LaterItemCardProps = {
  item: SynaraLaterItem;
};

function LaterItemCard({ item }: LaterItemCardProps) {
  const { t } = useTranslation();
  const mx = useMatrixClient();
  const { navigateRoom } = useRoomNavigate();
  const [editingDue, setEditingDue] = useState(false);
  const [customDue, setCustomDue] = useState(() => toDateTimeLocal(item.dueTs));
  const room = mx.getRoom(item.roomId);
  const event = room?.findEventById(item.eventId);
  const sender = event?.getSender() ?? undefined;
  const member = sender ? room?.getMember(sender) : undefined;
  const senderName =
    (sender && room && getMemberDisplayName(room, sender)) ??
    member?.name ??
    (sender ? getMxIdLocalPart(sender) : undefined) ??
    sender;
  const dueLabel = formatDue(item.dueTs);
  const eventContent = event?.getContent() as
    | {
        body?: unknown;
        msgtype?: unknown;
      }
    | undefined;
  const preview = getEventPreview(event?.getType(), eventContent, item.eventId);

  const openEventId = getThreadRootEventId(event) ?? item.eventId;
  const handleOpen = () => navigateRoom(item.roomId, openEventId);
  const handleDone = () => {
    void completeLaterWithNativeOwner(item.id).catch(() => undefined);
  };
  const handleSnooze = (dueTs: number) => {
    void snoozeLaterWithNativeOwner(item.id, dueTs).catch(() => undefined);
  };
  const handleSaveCustomDue = () => {
    const dueTs = fromDateTimeLocal(customDue);
    if (dueTs) {
      handleSnooze(dueTs);
      setEditingDue(false);
    }
  };
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(9, 0, 0, 0);

  return (
    <SequenceCard
      variant="SurfaceVariant"
      direction="Column"
      gap="300"
      style={{ padding: config.space.S400 }}
    >
      <Box justifyContent="SpaceBetween" gap="300" alignItems="Start">
        <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0 }}>
          <Box gap="200" alignItems="Center">
            <Text size="H5" truncate>
              {room?.name ?? item.roomId}
            </Text>
            <Chip
              variant={item.kind === 'reminder' ? 'Primary' : 'Surface'}
              radii="Pill"
              outlined
              before={
                <Icon
                  size="50"
                  src={item.kind === 'reminder' ? Icons.RecentClock : Icons.Bookmark}
                />
              }
            >
              <Text size="L400">
                {item.kind === 'reminder'
                  ? t('modernization.later.kind_reminder', 'Reminder')
                  : t('modernization.later.kind_saved', 'Saved')}
              </Text>
            </Chip>
            {item.completedAt && (
              <Chip variant="Success" radii="Pill" outlined>
                <Text size="L400">{t('modernization.later.done', 'Done')}</Text>
              </Chip>
            )}
            {dueLabel && (
              <Chip
                variant={item.dueTs && item.dueTs <= Date.now() ? 'Success' : 'Surface'}
                radii="Pill"
                outlined
              >
                <Text size="L400">{dueLabel}</Text>
              </Chip>
            )}
          </Box>
          {senderName && (
            <Text size="T200" priority="300" truncate>
              {senderName}
            </Text>
          )}
        </Box>
        <Box shrink="No" gap="200">
          <Button size="300" variant="Secondary" onClick={handleOpen}>
            <Text size="B300">{t('modernization.later.open', 'Open')}</Text>
          </Button>
          {!item.completedAt && (
            <Button size="300" variant="Success" fill="None" onClick={handleDone}>
              <Text size="B300">{t('modernization.later.done', 'Done')}</Text>
            </Button>
          )}
        </Box>
      </Box>
      <Text style={{ maxWidth: toRem(720) }} priority="300">
        {preview}
      </Text>
      {!item.completedAt && (
        <Box gap="200" wrap="Wrap" alignItems="Center">
          <Button
            size="300"
            variant="Secondary"
            fill="None"
            onClick={() => handleSnooze(Date.now() + 20 * 60_000)}
          >
            <Text size="B300">{t('modernization.later.snooze_20m', 'Snooze 20 min')}</Text>
          </Button>
          <Button
            size="300"
            variant="Secondary"
            fill="None"
            onClick={() => handleSnooze(Date.now() + 60 * 60_000)}
          >
            <Text size="B300">{t('modernization.later.snooze_1h', 'Snooze 1 hour')}</Text>
          </Button>
          <Button
            size="300"
            variant="Secondary"
            fill="None"
            onClick={() => handleSnooze(tomorrow.getTime())}
          >
            <Text size="B300">{t('modernization.later.snooze_tomorrow', 'Tomorrow')}</Text>
          </Button>
          <Button
            size="300"
            variant="Secondary"
            fill="None"
            onClick={() => setEditingDue((open) => !open)}
          >
            <Text size="B300">{t('modernization.later.edit_due', 'Edit due date')}</Text>
          </Button>
        </Box>
      )}
      {editingDue && !item.completedAt && (
        <Box
          as="form"
          gap="200"
          alignItems="Center"
          onSubmit={(evt: React.FormEvent<HTMLFormElement>) => {
            evt.preventDefault();
            handleSaveCustomDue();
          }}
        >
          <input
            type="datetime-local"
            value={customDue}
            aria-label={t('modernization.later.custom_due_aria_label', 'Custom reminder due date')}
            onChange={(evt) => setCustomDue(evt.currentTarget.value)}
          />
          <Button type="submit" size="300" variant="Primary">
            <Text size="B300">{t('modernization.later.save_due', 'Save due date')}</Text>
          </Button>
        </Box>
      )}
    </SequenceCard>
  );
}

export function Later() {
  const { t } = useTranslation();
  const screenSize = useScreenSizeContext();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [showDone, setShowDone] = useState(false);
  const laterContent = useAtomValue(laterContentAtom);
  const sortedItems = useMemo(() => getSortedLaterItems(laterContent), [laterContent]);
  const activeItems = sortedItems.filter((item) => !item.completedAt);
  const doneItems = sortedItems.filter((item) => !!item.completedAt);
  const items = showDone ? doneItems : activeItems;
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 190,
    overscan: 6,
  });
  const virtualItems = virtualizer.getVirtualItems();

  const handleClearDone = () => {
    void clearCompletedLaterWithNativeOwner().catch(() => undefined);
  };

  return (
    <Page>
      <PageHeader balance>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" basis="No">
            {screenSize === ScreenSize.Mobile && (
              <BackRouteHandler>
                {(onBack) => (
                  <IconButton onClick={onBack}>
                    <Icon src={Icons.ArrowLeft} />
                  </IconButton>
                )}
              </BackRouteHandler>
            )}
          </Box>
          <Box alignItems="Center" gap="200">
            {screenSize !== ScreenSize.Mobile && <Icon size="400" src={Icons.Bookmark} />}
            <Text size="H3" truncate>
              {t('modernization.later.title', 'Later')}
            </Text>
          </Box>
          <Box grow="Yes" basis="No" justifyContent="End">
            <Button
              size="300"
              variant={showDone ? 'Primary' : 'Secondary'}
              fill={showDone ? 'Solid' : 'None'}
              onClick={() => setShowDone((shown) => !shown)}
            >
              <Text size="B300">
                {showDone
                  ? t('modernization.later.show_active', 'Show active')
                  : t('modernization.later.show_done', 'Done {{count}}', {
                      count: doneItems.length,
                    })}
              </Text>
            </Button>
            {doneItems.length > 0 && (
              <Button size="300" variant="Secondary" fill="None" onClick={handleClearDone}>
                <Text size="B300">
                  {t('modernization.later.clear_completed', 'Clear completed')}
                </Text>
              </Button>
            )}
          </Box>
        </Box>
      </PageHeader>
      <Scroll ref={scrollRef} hideTrack visibility="Hover">
        <PageContent>
          <PageContentCenter>
            <Box direction="Column" gap="300">
              <span data-spacing-node />
              {items.length > 0 && (
                <div
                  style={{
                    position: 'relative',
                    height: virtualizer.getTotalSize(),
                  }}
                >
                  {virtualItems.map((virtualItem) => {
                    const item = items[virtualItem.index];
                    if (!item) return null;
                    return (
                      <VirtualTile
                        virtualItem={virtualItem}
                        style={{ paddingBottom: config.space.S300 }}
                        ref={virtualizer.measureElement}
                        key={item.id}
                      >
                        <LaterItemCard item={item} />
                      </VirtualTile>
                    );
                  })}
                </div>
              )}
              {items.length === 0 && (
                <SequenceCard
                  variant="SurfaceVariant"
                  direction="Column"
                  gap="200"
                  style={{ padding: config.space.S400 }}
                >
                  <Text>
                    {showDone
                      ? t('modernization.later.empty_done', 'No completed Later items.')
                      : t('modernization.later.empty', 'No saved messages or reminders.')}
                  </Text>
                  <Text size="T200" priority="300">
                    {t(
                      'modernization.later.empty_description',
                      'Use a message menu to save something for later or set a reminder.'
                    )}
                  </Text>
                </SequenceCard>
              )}
            </Box>
          </PageContentCenter>
        </PageContent>
      </Scroll>
    </Page>
  );
}
