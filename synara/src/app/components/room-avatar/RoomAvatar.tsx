import { AvatarFallback, AvatarImage, Icon, Icons, color } from 'folds';
import React, {
  ComponentProps,
  ReactEventHandler,
  ReactNode,
  forwardRef,
  useEffect,
  useState,
} from 'react';
import * as css from './RoomAvatar.css';
import { getRoomIconSrc } from '../../utils/room';
import colorMXID from '../../../util/colorMXID';
import type { RoomJoinRulePresentation } from '../../features/matrix-dto/roomJoinRule';
import { useNativeMatrixMediaSrc } from '../../hooks/useNativeMatrixMediaSrc';

type RoomAvatarProps = {
  roomId: string;
  src?: string;
  alt?: string;
  fallbackBackground?: string;
  fallbackColor?: string;
  renderFallback: () => ReactNode;
};
export function RoomAvatar({
  roomId,
  src,
  alt,
  fallbackBackground,
  fallbackColor,
  renderFallback,
}: RoomAvatarProps) {
  const resolvedSrc = useNativeMatrixMediaSrc(src);
  const [error, setError] = useState(false);

  useEffect(() => {
    setError(false);
  }, [src, resolvedSrc]);

  const handleLoad: ReactEventHandler<HTMLImageElement> = (evt) => {
    evt.currentTarget.setAttribute('data-image-loaded', 'true');
  };

  if (!resolvedSrc || error) {
    return (
      <AvatarFallback
        style={{
          backgroundColor: fallbackBackground ?? colorMXID(roomId ?? ''),
          color: fallbackColor ?? color.Surface.Container,
          textTransform: 'uppercase',
        }}
        className={css.RoomAvatar}
      >
        {renderFallback()}
      </AvatarFallback>
    );
  }

  return (
    <AvatarImage
      className={css.RoomAvatar}
      src={resolvedSrc}
      alt={alt}
      onError={() => setError(true)}
      onLoad={handleLoad}
      draggable={false}
    />
  );
}

export const RoomIcon = forwardRef<
  SVGSVGElement,
  Omit<ComponentProps<typeof Icon>, 'src'> & {
    joinRule?: RoomJoinRulePresentation | null;
    roomType?: string;
  }
>(({ joinRule, roomType, ...props }, ref) => (
  <Icon src={getRoomIconSrc(Icons, roomType, joinRule)} {...props} ref={ref} />
));
