import React, { FormEventHandler, ReactNode, useMemo } from 'react';
import { Box, Button, Chip, Icon, Icons, Input, Text } from 'folds';
import * as css from './style.css';
import { PackImageReader } from '../../plugins/custom-emoji';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { SettingTile } from '../setting-tile';
import { useObjectURL } from '../../hooks/useObjectURL';
import { createUploadAtom, TUploadAtom } from '../../state/upload';
import { replaceSpaceWithDash } from '../../utils/common';
import { resolveOptionalMatrixMediaUrl } from '../../matrix/media';

type ImageTileProps = {
  defaultShortcode: string;
  useAuthentication: boolean;
  image: PackImageReader;
  canEdit?: boolean;
  onEdit?: (defaultShortcode: string, image: PackImageReader) => void;
  deleted?: boolean;
  onDeleteToggle?: (defaultShortcode: string) => void;
};
export function ImageTile({
  defaultShortcode,
  image,
  useAuthentication,
  canEdit,
  onEdit,
  onDeleteToggle,
  deleted,
}: ImageTileProps) {
  const mx = useMatrixClient();
  return (
    <SettingTile
      before={
        <img
          className={css.ImagePackImage}
          src={resolveOptionalMatrixMediaUrl(mx, image.url, { useAuthentication }) ?? ''}
          alt={image.shortcode}
          loading="lazy"
        />
      }
      title={
        deleted ? (
          <span className={css.DeleteImageShortcode}>{image.shortcode}</span>
        ) : (
          image.shortcode
        )
      }
      description={image.body}
      after={
        canEdit ? (
          <Box shrink="No" alignItems="Center" gap="200">
            <Chip
              variant={deleted ? 'Critical' : 'Secondary'}
              fill="None"
              radii="Pill"
              onClick={() => onDeleteToggle?.(defaultShortcode)}
            >
              {deleted ? <Text size="B300">Undo</Text> : <Icon size="50" src={Icons.Delete} />}
            </Chip>
            {!deleted && (
              <Chip
                variant="Secondary"
                radii="Pill"
                onClick={() => onEdit?.(defaultShortcode, image)}
              >
                <Text size="B300">Edit</Text>
              </Chip>
            )}
          </Box>
        ) : undefined
      }
    />
  );
}

type ImageTileUploadProps = {
  file: File;
  children: (uploadAtom: TUploadAtom) => ReactNode;
};
export function ImageTileUpload({ file, children }: ImageTileUploadProps) {
  const url = useObjectURL(file);
  const uploadAtom = useMemo(() => createUploadAtom(file), [file]);

  return (
    <SettingTile before={<img className={css.ImagePackImage} src={url} alt={file.name} />}>
      {children(uploadAtom)}
    </SettingTile>
  );
}

type ImageTileEditProps = {
  defaultShortcode: string;
  useAuthentication: boolean;
  image: PackImageReader;
  onCancel: (shortcode: string) => void;
  onSave: (shortcode: string, image: PackImageReader) => void;
};
export function ImageTileEdit({
  defaultShortcode,
  useAuthentication,
  image,
  onCancel,
  onSave,
}: ImageTileEditProps) {
  const mx = useMatrixClient();
  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();

    const target = evt.target as HTMLFormElement | undefined;
    const shortcodeInput = target?.shortcodeInput as HTMLInputElement | undefined;
    const bodyInput = target?.bodyInput as HTMLTextAreaElement | undefined;
    if (!shortcodeInput || !bodyInput) return;

    const shortcode = replaceSpaceWithDash(shortcodeInput.value.trim());
    const body = bodyInput.value.trim() || undefined;
    if (!shortcode) return;

    if (shortcode === image.shortcode && body === image.body) {
      onCancel(defaultShortcode);
      return;
    }

    const { url: _url, ...existingContent } = image.content;
    void _url;
    const imageReader = new PackImageReader(shortcode, image.url, {
      ...existingContent,
      body,
    });

    onSave(defaultShortcode, imageReader);
  };

  return (
    <SettingTile
      before={
        <img
          className={css.ImagePackImage}
          src={resolveOptionalMatrixMediaUrl(mx, image.url, { useAuthentication }) ?? ''}
          alt={image.shortcode}
          loading="lazy"
        />
      }
    >
      <Box as="form" onSubmit={handleSubmit} direction="Column" gap="200">
        <Box direction="Column" className={css.ImagePackImageInputs}>
          <Input
            before={<Text size="L400">Shortcode:</Text>}
            defaultValue={image.shortcode}
            name="shortcodeInput"
            variant="Secondary"
            size="300"
            radii="0"
            required
            autoFocus
          />
          <Input
            before={<Text size="L400">Body:</Text>}
            defaultValue={image.body}
            name="bodyInput"
            variant="Secondary"
            size="300"
            radii="0"
          />
        </Box>
        <Box gap="200">
          <Box grow="Yes" />
          <Button type="submit" variant="Success" size="300" radii="300">
            <Text size="B300">Save</Text>
          </Button>
          <Button
            type="reset"
            variant="Secondary"
            fill="Soft"
            size="300"
            radii="300"
            onClick={() => onCancel(defaultShortcode)}
          >
            <Text size="B300">Cancel</Text>
          </Button>
        </Box>
      </Box>
    </SettingTile>
  );
}
