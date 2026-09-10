import React from 'react';
import { useNavigate, useRouteError } from 'react-router-dom';
import { RouteErrorFallback } from '../components/app-error';
import { getHomePath } from './pathUtils';

export function RouteError() {
  const error = useRouteError();
  const navigate = useNavigate();

  return (
    <RouteErrorFallback
      error={error}
      onRetry={() => window.location.reload()}
      onHome={() => navigate(getHomePath())}
    />
  );
}
