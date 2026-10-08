import React, { useEffect, useMemo, useState } from 'react';
import { Text, Switch, Button, color } from 'folds';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import { getNotificationState, usePermissionState } from '../../../hooks/usePermission';
import {
  getPlatformNotificationPermission,
  requestPlatformNotificationPermission,
  supportsPlatformSystemNotifications,
} from '../../../platform';
import type { PlatformNotificationPermission } from '../../../platform';
import { SettingsSection } from '../../../components/settings-layout';

export function SystemNotification() {
  const browserNotifPermission = usePermissionState('notifications', getNotificationState());
  const platformNotifications = useMemo(() => supportsPlatformSystemNotifications(), []);
  const [platformNotifPermission, setPlatformNotifPermission] =
    useState<PlatformNotificationPermission>();
  const [showNotifications, setShowNotifications] = useSetting(settingsAtom, 'showNotifications');
  const [isNotificationSounds, setIsNotificationSounds] = useSetting(
    settingsAtom,
    'isNotificationSounds'
  );

  useEffect(() => {
    if (!platformNotifications) return undefined;

    let disposed = false;
    getPlatformNotificationPermission().then((permission) => {
      if (!disposed) setPlatformNotifPermission(permission);
    });

    return () => {
      disposed = true;
    };
  }, [platformNotifications]);

  const notifPermission = platformNotifications
    ? (platformNotifPermission ?? 'prompt')
    : browserNotifPermission;

  const [isRequestingPermission, setIsRequestingPermission] = useState(false);
  const [permissionMessage, setPermissionMessage] = useState<string | null>(null);

  const requestNotificationPermission = async () => {
    if (isRequestingPermission) return;
    setIsRequestingPermission(true);
    setPermissionMessage(null);
    try {
      if (platformNotifications) {
        const permission = await requestPlatformNotificationPermission();
        setPlatformNotifPermission(permission);
        if (permission === 'granted' && notifPermission !== 'granted') setShowNotifications(true);
        setPermissionMessage(
          'Notification permission checked. Critical Alerts depend on macOS permission and support in this build.'
        );
      } else if ('Notification' in window) {
        const permission = await window.Notification.requestPermission();
        if (permission === 'granted') setShowNotifications(true);
      }
    } catch {
      setPermissionMessage(
        'Notification permission could not be requested. Check your system notification settings.'
      );
    } finally {
      setIsRequestingPermission(false);
    }
  };

  return (
    <SettingsSection title="System">
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Desktop Notifications"
          description={
            notifPermission === 'denied' ? (
              <Text as="span" style={{ color: color.Critical.Main }} size="T200">
                {platformNotifications
                  ? 'Notification permission is blocked. Allow Synara in your system notification settings.'
                  : 'Notification' in window
                    ? 'Notification permission is blocked. Please allow notification permission from browser address bar.'
                    : 'Notifications are not supported by the system.'}
              </Text>
            ) : (
              <span>Show desktop notifications when message arrive.</span>
            )
          }
          after={
            notifPermission === 'prompt' ? (
              <Button
                className={SettingsQuietControl}
                size="300"
                radii="300"
                variant="Primary"
                fill="Soft"
                disabled={isRequestingPermission}
                onClick={requestNotificationPermission}
              >
                <Text size="B300">Enable</Text>
              </Button>
            ) : (
              <Switch
                disabled={notifPermission !== 'granted'}
                value={showNotifications}
                onChange={setShowNotifications}
              />
            )
          }
        />
        {platformNotifications && notifPermission !== 'prompt' && (
          <SettingTile
            title="Notification Permission"
            description="On supported macOS builds, requesting permission also lets you opt into Critical Alerts for verified agent approvals. Your macOS notification settings control which alerts can be delivered."
          >
            <Button
              className={SettingsQuietControl}
              size="300"
              radii="300"
              variant="Secondary"
              fill="Soft"
              disabled={isRequestingPermission}
              onClick={requestNotificationPermission}
            >
              <Text size="B300">
                {isRequestingPermission ? 'Requesting…' : 'Request notification permission'}
              </Text>
            </Button>
          </SettingTile>
        )}
        {permissionMessage && (
          <Text size="T200" role="status">
            {permissionMessage}
          </Text>
        )}
      </SequenceCard>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Notification Sound"
          description="Play sound when new message arrive."
          after={<Switch value={isNotificationSounds} onChange={setIsNotificationSounds} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}
