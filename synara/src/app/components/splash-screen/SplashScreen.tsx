import { Box, Text } from 'folds';
import React, { ReactNode } from 'react';
import * as css from './SplashScreen.css';

type SplashScreenProps = {
  children: ReactNode;
};
export function SplashScreen({ children }: SplashScreenProps) {
  return (
    <Box className={css.SplashScreen} direction="Column">
      {children}
      <Box
        className={css.SplashScreenFooter}
        shrink="No"
        alignItems="Center"
        justifyContent="Center"
      >
        <Text size="H2" align="Center">
          Synara
        </Text>
      </Box>
    </Box>
  );
}
