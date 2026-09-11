import React from 'react';
import { Box, Button, Icon, IconButton, Icons, Text, color } from 'folds';
import { Page, PageContent, PageHeader } from '../page';
import { ContainerColor } from '../../styles/ContainerColor.css';
import { formatUnknownError } from './errorMessage';

type AppErrorFallbackProps = {
  error: unknown;
  title?: string;
  description?: string;
  onClose?: () => void;
  onRetry?: () => void;
  onHome?: () => void;
};

export function AppErrorFallback({
  error,
  title = 'Something went wrong',
  description,
  onClose,
  onRetry,
  onHome,
}: AppErrorFallbackProps) {
  const message = formatUnknownError(error);

  return (
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              {title}
            </Text>
          </Box>
          {onClose && (
            <Box shrink="No">
              <IconButton onClick={onClose} variant="Surface" aria-label="Close">
                <Icon src={Icons.Cross} />
              </IconButton>
            </Box>
          )}
        </Box>
      </PageHeader>
      <Box grow="Yes" direction="Column">
        <PageContent>
          <Box direction="Column" gap="400">
            {description && (
              <Text size="T300" priority="400">
                {description}
              </Text>
            )}
            <Text size="T300" style={{ color: color.Critical.Main }}>
              {message}
            </Text>
            <Box gap="200" wrap="Wrap">
              {onRetry && (
                <Button variant="Primary" onClick={onRetry}>
                  <Text size="B400">Try again</Text>
                </Button>
              )}
              {onHome && (
                <Button variant="Secondary" fill="Soft" onClick={onHome}>
                  <Text size="B400">Go home</Text>
                </Button>
              )}
              {onClose && (
                <Button variant="Secondary" fill="Soft" onClick={onClose}>
                  <Text size="B400">Close</Text>
                </Button>
              )}
            </Box>
          </Box>
        </PageContent>
      </Box>
    </Page>
  );
}

type RouteErrorFallbackProps = {
  error: unknown;
  onRetry: () => void;
  onHome: () => void;
};

export function RouteErrorFallback({ error, onRetry, onHome }: RouteErrorFallbackProps) {
  return (
    <Box
      grow="Yes"
      className={ContainerColor({ variant: 'Background' })}
      style={{ minHeight: '100%' }}
    >
      <AppErrorFallback
        error={error}
        description="This screen ran into a problem. You can reload or return home."
        onRetry={onRetry}
        onHome={onHome}
      />
    </Box>
  );
}
