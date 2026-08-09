import {
  type Capability,
  type ISendDelayedEventDetails,
  type ISendEventDetails,
  type IReadEventRelationsResult,
  type IRoomEvent,
  WidgetDriver,
  type IWidgetApiErrorResponseDataDetails,
  type ISearchUserDirectoryResult,
  type IGetMediaConfigResult,
  type UpdateDelayedEventAction,
  OpenIDRequestState,
  SimpleObservable,
  IOpenIDUpdate,
} from 'matrix-widget-api';
import type { useMatrixClient } from '../../hooks/useMatrixClient';
import type { MatrixEventReading } from '../../utils/room';

/** Timeline event with the widget check used here. */
type WidgetEventReading = MatrixEventReading & { isState(): boolean };

/** The live client type (type-only; matches useMatrixClient). */
type LocalMx = ReturnType<typeof useMatrixClient>;
/** Widget event content projection. */
type WidgetContent = Record<string, any>;
/** Delayed-event send response projection. */
type SendDelayedWidgetResponse = { [key: string]: any };

const hasWidgetApiErrorData = (error: unknown): error is { asWidgetApiErrorData(): unknown } =>
  typeof (error as { asWidgetApiErrorData?: unknown } | null)?.asWidgetApiErrorData === 'function';
import { getCallCapabilities } from './utils';
import { uploadCallWidgetFileWithNativeOwner } from './nativeCallMediaUploadOwner';
import {
  downloadFileWithNativeOwner,
  getMediaConfigWithNativeOwner,
} from './nativeCallWidgetMediaOwner';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { getNativeRoomListSnapshot } from '../../state/room-list/roomList';
import { getLoadedLiveTimelineEvents, getRoomCurrentState } from '../../utils/timelineLifecycle';
import { getKnownRoomsFromNativeSnapshot } from './nativeCallWidgetOwner';

export class CallWidgetDriver extends WidgetDriver {
  private allowedCapabilities: Set<Capability>;

  private readonly mx: LocalMx;

  public constructor(mx: LocalMx, private inRoomId: string) {
    super();
    this.mx = mx;

    const deviceId = mx.getDeviceId();
    if (!deviceId) throw new Error('Failed to initialize CallWidgetDriver! Device ID not found.');

    this.allowedCapabilities = getCallCapabilities(inRoomId, mx.getSafeUserId(), deviceId);
  }

  public async validateCapabilities(requested: Set<Capability>): Promise<Set<Capability>> {
    const allow = Array.from(requested).filter((cap) => this.allowedCapabilities.has(cap));
    return new Set(allow);
  }

  public async sendEvent(
    eventType: string,
    content: WidgetContent,
    stateKey: string | null = null,
    targetRoomId: string | null = null
  ): Promise<ISendEventDetails> {
    const client = this.mx;
    const roomId = targetRoomId || this.inRoomId;

    if (!client || !roomId) throw new Error('Not in a room or not attached to a client');

    let r: { event_id?: string } | null;
    if (typeof stateKey === 'string') {
      r = await client.sendStateEvent(roomId, eventType as never, content as never, stateKey);
    } else if (eventType === 'm.room.redaction') {
      // special case: extract the `redacts` property and call redact
      r = await client.redactEvent(roomId, content.redacts);
    } else {
      r = await client.sendEvent(roomId, eventType as never, content as never);
    }

    return { roomId, eventId: r?.event_id ?? '' };
  }

  public async sendDelayedEvent(
    delay: number | null,
    parentDelayId: string | null,
    eventType: string,
    content: WidgetContent,
    stateKey: string | null = null,
    targetRoomId: string | null = null
  ): Promise<ISendDelayedEventDetails> {
    const client = this.mx;
    const roomId = targetRoomId || this.inRoomId;

    if (!client || !roomId) throw new Error('Not in a room or not attached to a client');

    let delayOpts;
    if (delay !== null) {
      delayOpts = {
        delay,
        ...(parentDelayId !== null && { parent_delay_id: parentDelayId }),
      };
    } else if (parentDelayId !== null) {
      delayOpts = {
        parent_delay_id: parentDelayId,
      };
    } else {
      throw new Error('Must provide at least one of delay or parentDelayId');
    }

    let r: SendDelayedWidgetResponse | null;
    if (stateKey !== null) {
      // state event
      r = await client._unstable_sendDelayedStateEvent(
        roomId,
        delayOpts,
        eventType as never,
        content as never,
        stateKey
      );
    } else {
      // message event
      r = await client._unstable_sendDelayedEvent(
        roomId,
        delayOpts,
        null,
        eventType as never,
        content as never
      );
    }

    return {
      roomId,
      delayId: r?.delay_id ?? '',
    };
  }

  public async updateDelayedEvent(
    delayId: string,
    action: UpdateDelayedEventAction
  ): Promise<void> {
    const client = this.mx;

    if (!client) throw new Error('Not in a room or not attached to a client');

    await client._unstable_updateDelayedEvent(delayId, action);
  }

  public async sendToDevice(
    eventType: string,
    encrypted: boolean,
    contentMap: { [userId: string]: { [deviceId: string]: object } }
  ): Promise<void> {
    const client = this.mx;

    if (encrypted) {
      const crypto = client.getCrypto();
      if (!crypto) throw new Error('E2EE not enabled');

      // attempt to re-batch these up into a single request
      const invertedContentMap: { [content: string]: { userId: string; deviceId: string }[] } = {};

      // eslint-disable-next-line no-restricted-syntax
      for (const userId of Object.keys(contentMap)) {
        const userContentMap = contentMap[userId];
        // eslint-disable-next-line no-restricted-syntax
        for (const deviceId of Object.keys(userContentMap)) {
          const content = userContentMap[deviceId];
          const stringifiedContent = JSON.stringify(content);
          invertedContentMap[stringifiedContent] = invertedContentMap[stringifiedContent] || [];
          invertedContentMap[stringifiedContent].push({ userId, deviceId });
        }
      }

      await Promise.all(
        Object.entries(invertedContentMap).map(async ([stringifiedContent, recipients]) => {
          const batch = await crypto.encryptToDeviceMessages(
            eventType,
            recipients,
            JSON.parse(stringifiedContent)
          );

          await client.queueToDevice(batch);
        })
      );
    } else {
      await client.queueToDevice({
        eventType,
        batch: Object.entries(contentMap).flatMap(([userId, userContentMap]) =>
          Object.entries(userContentMap).map(([deviceId, content]) => ({
            userId,
            deviceId,
            payload: content,
          }))
        ),
      });
    }
  }

  public async readRoomTimeline(
    roomId: string,
    eventType: string,
    msgtype: string | undefined,
    stateKey: string | undefined,
    limit: number,
    since: string | undefined
  ): Promise<IRoomEvent[]> {
    const safeLimit =
      limit > 0 ? Math.min(limit, Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER; // relatively arbitrary

    const room = this.mx.getRoom(roomId);
    if (room === null) return [];
    const results: WidgetEventReading[] = [];
    const events = getLoadedLiveTimelineEvents(room) as WidgetEventReading[];

    for (let i = events.length - 1; i >= 0; i -= 1) {
      const ev = events[i];
      if (results.length >= safeLimit) break;
      if (since !== undefined && ev.getId() === since) break;

      if (
        ev.getType() === eventType &&
        !ev.isState() &&
        (eventType !== 'm.room.message' || !msgtype || msgtype === ev.getContent().msgtype) &&
        (ev.getStateKey() === undefined || stateKey === undefined || ev.getStateKey() === stateKey)
      ) {
        results.push(ev);
      }
    }

    return results.map((e) => e.getEffectiveEvent?.() as IRoomEvent);
  }

  public async askOpenID(observer: SimpleObservable<IOpenIDUpdate>): Promise<void> {
    return observer.update({
      state: OpenIDRequestState.Allowed,
      token: (await this.mx.getOpenIdToken()) ?? undefined,
    });
  }

  public async readRoomState(
    roomId: string,
    eventType: string,
    stateKey: string | undefined
  ): Promise<IRoomEvent[]> {
    const room = this.mx.getRoom(roomId);
    if (room === null) return [];
    const state = getRoomCurrentState(room);
    if (state === undefined) return [];

    if (stateKey === undefined)
      return (state.getStateEvents(eventType) as WidgetEventReading[]).map(
        (e) => e.getEffectiveEvent?.() as IRoomEvent
      );
    const event = state.getStateEvents(eventType, stateKey) as MatrixEventReading | null;
    return event === null ? [] : [event.getEffectiveEvent?.() as IRoomEvent];
  }

  public async readEventRelations(
    eventId: string,
    roomId?: string,
    relationType?: string,
    eventType?: string,
    from?: string,
    to?: string,
    limit?: number,
    direction?: 'f' | 'b'
  ): Promise<IReadEventRelationsResult> {
    const client = this.mx;
    const dir = direction === 'f' ? 'f' : direction === 'b' ? 'b' : undefined;
    const targetRoomId = roomId ?? this.inRoomId ?? undefined;

    if (typeof targetRoomId !== 'string') {
      throw new Error('Error while reading the current room');
    }

    const { events, nextBatch, prevBatch } = await client.relations(
      targetRoomId,
      eventId,
      relationType ?? null,
      eventType ?? null,
      { from, to, limit, dir: dir as never }
    );

    return {
      chunk: events.map((e) => e.getEffectiveEvent?.() as IRoomEvent),
      nextBatch: typeof nextBatch === 'string' ? nextBatch : undefined,
      prevBatch: typeof prevBatch === 'string' ? prevBatch : undefined,
    };
  }

  public async searchUserDirectory(
    searchTerm: string,
    limit?: number
  ): Promise<ISearchUserDirectoryResult> {
    const client = this.mx;

    const { limited, results } = await client.searchUserDirectory({ term: searchTerm, limit });

    return {
      limited,
      results: results.map((r) => ({
        userId: r.user_id,
        displayName: r.display_name,
        avatarUrl: r.avatar_url,
      })),
    };
  }

  public async getMediaConfig(): Promise<IGetMediaConfigResult> {
    return getMediaConfigWithNativeOwner(isSynaraDesktop(), (command, args) =>
      invokeDesktopWithAvailability(command, args)
    );
  }

  public async uploadFile(file: XMLHttpRequestBodyInit): Promise<{ contentUri: string }> {
    const client = this.mx;

    return uploadCallWidgetFileWithNativeOwner(
      file,
      isSynaraDesktop(),
      (command, args) => invokeDesktopWithAvailability(command, args),
      () => client.uploadContent(file)
    );
  }

  public async downloadFile(contentUri: string): Promise<{ file: XMLHttpRequestBodyInit }> {
    return downloadFileWithNativeOwner(contentUri, isSynaraDesktop(), (command, args) =>
      invokeDesktopWithAvailability(command, args)
    );
  }

  public getKnownRooms(): string[] {
    return getKnownRoomsFromNativeSnapshot(isSynaraDesktop(), getNativeRoomListSnapshot());
  }

  // eslint-disable-next-line class-methods-use-this
  public processError(error: unknown): IWidgetApiErrorResponseDataDetails | undefined {
    return hasWidgetApiErrorData(error)
      ? ({
          matrix_api_error:
            error.asWidgetApiErrorData() as unknown as IWidgetApiErrorResponseDataDetails,
        } as unknown as IWidgetApiErrorResponseDataDetails)
      : undefined;
  }
}
