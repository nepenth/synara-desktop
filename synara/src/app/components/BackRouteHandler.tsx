import { ReactNode, useCallback } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { getBackRoutePath } from './backRoutePath';

type BackRouteHandlerProps = {
  children: (onBack: () => void) => ReactNode;
};
export function BackRouteHandler({ children }: BackRouteHandlerProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const goBack = useCallback(
    () => navigate(getBackRoutePath(location.pathname, location.state)),
    [navigate, location.pathname, location.state]
  );
  return children(goBack);
}
