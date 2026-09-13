// Actual sidebar virtualizer and tiles, with deterministic room labels instead
// of live Matrix data. Exercises browser layout, resizing, and scroll geometry.
import React, { useCallback, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { configClass, varsClass } from 'folds';
import 'folds/dist/style.css';
import { useSharedScrollVirtualizer } from '../../src/app/hooks/useSharedScrollVirtualizer';
import { NotificationsFixture } from './NotificationsFixture';
import { VirtualTile } from '../../src/app/components/virtualizer/VirtualTile';

function RoomListFixture() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [favorites, setFavorites] = useState(10);
  const [reversed, setReversed] = useState(false);
  const [mounted, setMounted] = useState(true);
  const getItemKey = useCallback(
    (index: number) => `room-${reversed ? 99 - index : index}`,
    [reversed]
  );
  const { virtualizer, listRef, scrollMargin } = useSharedScrollVirtualizer(
    scrollRef,
    100,
    getItemKey
  );
  return (
    <main className={`${configClass} ${varsClass}`} style={{ fontFamily: 'sans-serif' }}>
      <h1>Room list scroll proof</h1>
      <p>Real sidebar virtualization with 100 rooms and adjustable preceding favorites.</p>
      <button type="button" onClick={() => setFavorites((value) => (value === 10 ? 20 : 10))}>
        Toggle favorites
      </button>
      <button type="button" onClick={() => setReversed((value) => !value)}>
        Reverse rooms
      </button>
      <button type="button" onClick={() => setMounted((value) => !value)}>
        Toggle list
      </button>
      {mounted && (
        <div
          ref={scrollRef}
          data-testid="room-scroll"
          style={{ height: 680, width: 340, overflowY: 'auto', border: '2px solid #555' }}
        >
          <div style={{ padding: 8 }}>
            <div style={{ height: 120 }}>
              Create Room
              <br />
              Join with Address
              <br />
              Message Search
            </div>
            <div style={{ height: 40 }}>Favorites</div>
            {Array.from({ length: favorites }, (_, index) => (
              <div key={index} style={{ height: 40 }}>
                Favorite {index + 1}
              </div>
            ))}
            <div style={{ height: 40 }}>Rooms</div>
            <div
              ref={listRef}
              data-testid="room-list"
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
                    data-room-index={item.index}
                    style={{
                      height: 40,
                      boxSizing: 'border-box',
                      borderBottom: '1px solid #ddd',
                      display: 'flex',
                      alignItems: 'center',
                    }}
                  >
                    # Project — {getItemKey(item.index)}
                  </div>
                </VirtualTile>
              ))}
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

createRoot(document.getElementById('root')!).render(
  new URLSearchParams(window.location.search).has('notifications') ? (
    <NotificationsFixture />
  ) : (
    <RoomListFixture />
  )
);
