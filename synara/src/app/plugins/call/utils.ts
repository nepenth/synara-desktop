import {
  type Capability,
  EventDirection,
  MatrixCapabilities,
  WidgetEventCapability,
} from 'matrix-widget-api';

export function getCallCapabilities(
  roomId: string,
  userId: string,
  deviceId: string
): Set<Capability> {
  const capabilities: Set<Capability> = new Set();

  capabilities.add(MatrixCapabilities.Screenshots);
  capabilities.add(MatrixCapabilities.AlwaysOnScreen);
  capabilities.add(MatrixCapabilities.MSC3846TurnServers);
  capabilities.add(MatrixCapabilities.MSC4157SendDelayedEvent);
  capabilities.add(MatrixCapabilities.MSC4157UpdateDelayedEvent);
  capabilities.add(`org.matrix.msc2762.timeline:${roomId}`);
  capabilities.add(`org.matrix.msc2762.state:${roomId}`);

  capabilities.add(
    WidgetEventCapability.forStateEvent(EventDirection.Receive, 'm.room.member').raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(EventDirection.Receive, 'org.matrix.msc3401.call').raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(EventDirection.Receive, 'm.room.encryption').raw
  );
  capabilities.add(WidgetEventCapability.forStateEvent(EventDirection.Receive, 'm.room.name').raw);

  capabilities.add(
    WidgetEventCapability.forStateEvent(
      EventDirection.Send,
      'org.matrix.msc3401.call.member',
      userId
    ).raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(
      EventDirection.Send,
      'org.matrix.msc3401.call.member',
      `_${userId}_${deviceId}_m.call`
    ).raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(
      EventDirection.Send,
      'org.matrix.msc3401.call.member',
      `${userId}_${deviceId}_m.call`
    ).raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(
      EventDirection.Send,
      'org.matrix.msc3401.call.member',
      `_${userId}_${deviceId}`
    ).raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(
      EventDirection.Send,
      'org.matrix.msc3401.call.member',
      `${userId}_${deviceId}`
    ).raw
  );

  capabilities.add(
    WidgetEventCapability.forStateEvent(EventDirection.Receive, 'org.matrix.msc3401.call.member')
      .raw
  );
  capabilities.add(
    WidgetEventCapability.forStateEvent(EventDirection.Receive, 'm.room.create').raw
  );

  capabilities.add(
    WidgetEventCapability.forRoomEvent(
      EventDirection.Receive,
      'org.matrix.msc4075.rtc.notification'
    ).raw
  );

  [
    'io.element.call.encryption_keys',
    'org.matrix.rageshake_request',
    'm.reaction',
    'm.room.redaction',
    'io.element.call.reaction',
    'org.matrix.msc4310.rtc.decline',
  ].forEach((type) => {
    capabilities.add(WidgetEventCapability.forRoomEvent(EventDirection.Send, type).raw);
    capabilities.add(WidgetEventCapability.forRoomEvent(EventDirection.Receive, type).raw);
  });

  [
    'm.call.invite',
    'm.call.candidates',
    'm.call.answer',
    'm.call.hangup',
    'm.call.reject',
    'm.call.select_answer',
    'm.call.negotiate',
    'm.call.sdp_stream_metadata_changed',
    'm.call.sdp_stream_metadata_changed_prefix',
    'm.call.replaces',
    'm.call.encryption_keys_prefix',
  ].forEach((type) => {
    capabilities.add(WidgetEventCapability.forToDeviceEvent(EventDirection.Send, type).raw);
    capabilities.add(WidgetEventCapability.forToDeviceEvent(EventDirection.Receive, type).raw);
  });

  return capabilities;
}
