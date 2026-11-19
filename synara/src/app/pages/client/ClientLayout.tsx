import React, { ReactNode } from 'react';
import { Box } from 'folds';

type ClientLayoutProps = {
  children: ReactNode;
};
/**
 * The signed-in client's frame. Navigation lives in each route's list column
 * (its header switcher and bottom bar), so there is no rail beside it.
 */
export function ClientLayout({ children }: ClientLayoutProps) {
  // Positioned, so the "What's new" card can sit in its corner.
  return (
    <Box grow="Yes" style={{ position: 'relative' }}>
      {children}
    </Box>
  );
}
