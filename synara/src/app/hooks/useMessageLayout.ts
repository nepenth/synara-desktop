import { useMemo } from 'react';
import { MessageLayout } from '../state/settings';

export type MessageLayoutItem = {
  name: string;
  layout: MessageLayout;
};

export const useMessageLayoutItems = (): MessageLayoutItem[] =>
  useMemo(
    () => [
      {
        layout: MessageLayout.Modern,
        name: 'Standard',
      },
      {
        layout: MessageLayout.Compact,
        name: 'Compact',
      },
      {
        layout: MessageLayout.Bubble,
        name: 'Stacked bubbles',
      },
    ],
    []
  );
