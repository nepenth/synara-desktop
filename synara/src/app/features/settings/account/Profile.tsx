import React, {
  ChangeEventHandler,
  FormEventHandler,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import {
  Box,
  Text,
  IconButton,
  Icon,
  Icons,
  Input,
  Avatar,
  Button,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Modal,
  Dialog,
  Header,
  config,
  Spinner,
} from 'folds';
import FocusTrap from 'focus-trap-react';
import { SequenceCard } from '../../../components/sequence-card';
import { SequenceCardStyle } from '../styles.css';
import { SettingTile } from '../../../components/setting-tile';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { UserProfile, useUserProfile } from '../../../hooks/useUserProfile';
import { getMxIdLocalPart } from '../../../utils/matrix';
import { UserAvatar } from '../../../components/user-avatar';
import { nameInitials } from '../../../utils/common';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useFilePicker } from '../../../hooks/useFilePicker';
import { useObjectURL } from '../../../hooks/useObjectURL';
import { stopPropagation } from '../../../utils/keyboard';
import { ImageEditor } from '../../../components/image-editor';
import { ModalWide } from '../../../styles/Modal.css';
import { createUploadAtom, UploadSuccess } from '../../../state/upload';
import { CompactUploadCardRenderer } from '../../../components/upload-card';
import { useCapabilities } from '../../../hooks/useCapabilities';
import {
  notifyOwnProfileChanged,
  setOwnAvatarNative,
  setOwnDisplayNameNative,
  uploadMediaNative,
} from './nativeProfile';
import { isSynaraDesktop } from '../../../utils/desktop';

type ProfileProps = {
  profile: UserProfile;
  userId: string;
};
function ProfileAvatar({ profile, userId }: ProfileProps) {
  const mx = useMatrixClient();
  const capabilities = useCapabilities();
  const [alertRemove, setAlertRemove] = useState(false);
  const disableSetAvatar = capabilities['m.set_avatar_url']?.enabled === false;

  const defaultDisplayName = profile.displayName ?? getMxIdLocalPart(userId) ?? userId;
  const avatarUrl = profile.avatarUrl;

  const [imageFile, setImageFile] = useState<File>();
  const imageFileURL = useObjectURL(imageFile);
  const [nativeUploading, setNativeUploading] = useState(false);
  const [nativeUploadError, setNativeUploadError] = useState(false);
  const uploadAtom = useMemo(() => {
    // Legacy web path only: desktop native uses uploadMediaNative below.
    if (imageFile && !isSynaraDesktop()) {
      return createUploadAtom(imageFile);
    }
    return undefined;
  }, [imageFile]);

  const pickFile = useFilePicker(setImageFile, false);

  const handleRemoveUpload = useCallback(() => {
    setImageFile(undefined);
    setNativeUploadError(false);
  }, []);

  const handleUploaded = useCallback(
    async (upload: UploadSuccess) => {
      const { mxc } = upload;
      const result = await setOwnAvatarNative(mxc);
      if (result === 'legacy') {
        await mx.setAvatarUrl(mxc);
      }
      notifyOwnProfileChanged();
      handleRemoveUpload();
    },
    [mx, handleRemoveUpload]
  );

  useEffect(() => {
    if (!imageFile || !isSynaraDesktop()) return;
    // Desktop native: fail-closed upload + set avatar without mx.uploadContent.
    let cancelled = false;
    (async () => {
      setNativeUploading(true);
      setNativeUploadError(false);
      try {
        const bytes = Array.from(new Uint8Array(await imageFile.arrayBuffer()));
        const mimeType = imageFile.type || 'image/png';
        const uploaded = await uploadMediaNative(mimeType, bytes);
        if (cancelled) return;
        if (uploaded === 'legacy') {
          // Not a native session (unexpected on desktop shell) — fail closed.
          setNativeUploadError(true);
          setNativeUploading(false);
          return;
        }
        const setResult = await setOwnAvatarNative(uploaded.mxc);
        if (cancelled) return;
        if (setResult === 'legacy') {
          setNativeUploadError(true);
          setNativeUploading(false);
          return;
        }
        notifyOwnProfileChanged();
        handleRemoveUpload();
        setNativeUploading(false);
      } catch {
        if (!cancelled) {
          setNativeUploadError(true);
          setNativeUploading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [imageFile, mx, handleRemoveUpload]);

  const handleRemoveAvatar = async () => {
    const result = await setOwnAvatarNative('');
    if (result === 'legacy') {
      await mx.setAvatarUrl('');
    }
    notifyOwnProfileChanged();
    setAlertRemove(false);
  };

  return (
    <SettingTile
      title={
        <Text as="span" size="L400">
          Avatar
        </Text>
      }
      after={
        <Avatar size="500" radii="300">
          <UserAvatar
            userId={userId}
            src={avatarUrl}
            renderFallback={() => <Text size="H4">{nameInitials(defaultDisplayName)}</Text>}
          />
        </Avatar>
      }
    >
      {nativeUploading ? (
        <Box gap="200" alignItems="Center">
          <Spinner size="300" />
          <Text size="T300">Uploading avatar…</Text>
          {nativeUploadError && (
            <Text size="T300" style={{ color: 'var(--folds-color-Critical-Main)' }}>
              Native avatar upload failed.
            </Text>
          )}
        </Box>
      ) : uploadAtom ? (
        <Box gap="200" direction="Column">
          <CompactUploadCardRenderer
            uploadAtom={uploadAtom}
            onRemove={handleRemoveUpload}
            onComplete={handleUploaded}
          />
        </Box>
      ) : (
        <Box gap="200">
          <Button
            onClick={() => pickFile('image/*')}
            size="300"
            variant="Secondary"
            fill="Soft"
            outlined
            radii="300"
            disabled={disableSetAvatar}
          >
            <Text size="B300">Upload</Text>
          </Button>
          {avatarUrl && (
            <Button
              size="300"
              variant="Critical"
              fill="None"
              radii="300"
              disabled={disableSetAvatar}
              onClick={() => setAlertRemove(true)}
            >
              <Text size="B300">Remove</Text>
            </Button>
          )}
        </Box>
      )}

      {imageFileURL && (
        <Overlay open={false} backdrop={<OverlayBackdrop />}>
          <OverlayCenter>
            <FocusTrap
              focusTrapOptions={{
                initialFocus: false,
                onDeactivate: handleRemoveUpload,
                clickOutsideDeactivates: true,
                escapeDeactivates: stopPropagation,
              }}
            >
              <Modal className={ModalWide} variant="Surface" size="500">
                <ImageEditor
                  name={imageFile?.name ?? 'Unnamed'}
                  url={imageFileURL}
                  requestClose={handleRemoveUpload}
                />
              </Modal>
            </FocusTrap>
          </OverlayCenter>
        </Overlay>
      )}

      <Overlay open={alertRemove} backdrop={<OverlayBackdrop />}>
        <OverlayCenter>
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              onDeactivate: () => setAlertRemove(false),
              clickOutsideDeactivates: true,
              escapeDeactivates: stopPropagation,
            }}
          >
            <Dialog variant="Surface">
              <Header
                style={{
                  padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                  borderBottomWidth: config.borderWidth.B300,
                }}
                variant="Surface"
                size="500"
              >
                <Box grow="Yes">
                  <Text size="H4">Remove Avatar</Text>
                </Box>
                <IconButton size="300" onClick={() => setAlertRemove(false)} radii="300">
                  <Icon src={Icons.Cross} />
                </IconButton>
              </Header>
              <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
                <Box direction="Column" gap="200">
                  <Text priority="400">Are you sure you want to remove profile avatar?</Text>
                </Box>
                <Button variant="Critical" onClick={handleRemoveAvatar}>
                  <Text size="B400">Remove</Text>
                </Button>
              </Box>
            </Dialog>
          </FocusTrap>
        </OverlayCenter>
      </Overlay>
    </SettingTile>
  );
}

function ProfileDisplayName({ profile, userId }: ProfileProps) {
  const mx = useMatrixClient();
  const capabilities = useCapabilities();
  const disableSetDisplayname = capabilities['m.set_displayname']?.enabled === false;

  const defaultDisplayName = profile.displayName ?? getMxIdLocalPart(userId) ?? userId;
  const [displayName, setDisplayName] = useState<string>(defaultDisplayName);

  const [changeState, changeDisplayName] = useAsyncCallback(
    useCallback(
      async (name: string) => {
        const result = await setOwnDisplayNameNative(name);
        if (result === 'legacy') {
          await mx.setDisplayName(name);
        }
        notifyOwnProfileChanged();
      },
      [mx]
    )
  );
  const changingDisplayName = changeState.status === AsyncStatus.Loading;

  useEffect(() => {
    setDisplayName(defaultDisplayName);
  }, [defaultDisplayName]);

  const handleChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    const name = evt.currentTarget.value;
    setDisplayName(name);
  };

  const handleReset = () => {
    setDisplayName(defaultDisplayName);
  };

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (changingDisplayName) return;

    const target = evt.target as HTMLFormElement | undefined;
    const displayNameInput = target?.displayNameInput as HTMLInputElement | undefined;
    const name = displayNameInput?.value;
    if (!name) return;

    changeDisplayName(name);
  };

  const hasChanges = displayName !== defaultDisplayName;
  return (
    <SettingTile
      title={
        <Text as="span" size="L400">
          Display Name
        </Text>
      }
    >
      <Box direction="Column" grow="Yes" gap="100">
        <Box
          as="form"
          onSubmit={handleSubmit}
          gap="200"
          aria-disabled={changingDisplayName || disableSetDisplayname}
        >
          <Box grow="Yes" direction="Column">
            <Input
              required
              name="displayNameInput"
              value={displayName}
              onChange={handleChange}
              variant="Secondary"
              radii="300"
              style={{ paddingRight: config.space.S200 }}
              readOnly={changingDisplayName || disableSetDisplayname}
              after={
                hasChanges &&
                !changingDisplayName && (
                  <IconButton
                    type="reset"
                    onClick={handleReset}
                    size="300"
                    radii="300"
                    variant="Secondary"
                  >
                    <Icon src={Icons.Cross} size="100" />
                  </IconButton>
                )
              }
            />
          </Box>
          <Button
            size="400"
            variant={hasChanges ? 'Success' : 'Secondary'}
            fill={hasChanges ? 'Solid' : 'Soft'}
            outlined
            radii="300"
            disabled={!hasChanges || changingDisplayName}
            type="submit"
          >
            {changingDisplayName && <Spinner variant="Success" fill="Solid" size="300" />}
            <Text size="B400">Save</Text>
          </Button>
        </Box>
      </Box>
    </SettingTile>
  );
}

export function Profile() {
  const mx = useMatrixClient();
  const userId = mx.getUserId()!;
  const profile = useUserProfile(userId);

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Profile</Text>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <ProfileAvatar userId={userId} profile={profile} />
        <ProfileDisplayName userId={userId} profile={profile} />
      </SequenceCard>
    </Box>
  );
}
