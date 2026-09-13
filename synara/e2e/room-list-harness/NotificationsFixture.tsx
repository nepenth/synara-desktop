// Use the production shared-scroller hook and measured notification-group tiles.
import React, { useCallback, useRef, useState } from 'react';
import { configClass, varsClass } from 'folds';
import { useSharedScrollVirtualizer } from '../../src/app/hooks/useSharedScrollVirtualizer';
import { VirtualTile } from '../../src/app/components/virtualizer/VirtualTile';

export function NotificationsFixture() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [wrapped, setWrapped] = useState(false);
  const getItemKey = useCallback(
    (index: number) => `!room-${index}:example.org:$event-${index}`,
    []
  );
  const { virtualizer, listRef, scrollMargin } = useSharedScrollVirtualizer(
    scrollRef,
    60,
    getItemKey,
    4
  );
  return (
    <main className={`${configClass} ${varsClass}`}>
      <h1>Notifications scroll proof</h1>
      <button type="button" onClick={() => setWrapped((value) => !value)}>
        Wrap filters
      </button>
      <div
        ref={scrollRef}
        data-testid="notification-scroll"
        style={{ height: 580, width: 540, overflowY: 'auto', border: '2px solid #555' }}
      >
        <div style={{ padding: 16 }}>
          <div style={{ height: wrapped ? 1840 : 1280 }} data-testid="notification-filter">
            Filter <button type="button">All Notifications</button>{' '}
            <button type="button">Highlighted</button>
          </div>
          <div
            ref={listRef}
            data-testid="notification-list"
            style={{ position: 'relative', height: virtualizer.getTotalSize() }}
          >
            {virtualizer.getVirtualItems().map((item) => (
              <VirtualTile
                key={item.key}
                virtualItem={item}
                scrollMargin={scrollMargin}
                ref={virtualizer.measureElement}
              >
                <div
                  data-notification-index={item.index}
                  style={{
                    height: [96, 160, 224][item.index % 3],
                    boxSizing: 'border-box',
                    padding: 16,
                    border: '1px solid #888',
                  }}
                >
                  Room {item.index + 1}: Notification group
                </div>
              </VirtualTile>
            ))}
          </div>
        </div>
      </div>
    </main>
  );
}
