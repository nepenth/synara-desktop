import { useCallback } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { APPROVALS_PATH } from '../pages/paths';
import { createApprovalsNavigationState } from '../routes/approvalsNavigation';

export const useNavigateToApprovals = () => {
  const navigate = useNavigate();
  const location = useLocation();
  return useCallback(
    () => navigate(APPROVALS_PATH, { state: createApprovalsNavigationState(location) }),
    [navigate, location]
  );
};
