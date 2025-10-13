import { useCallback, useMemo } from "react";
import { useLocation, useNavigate } from "react-router-dom";

export const useAboutModal = () => {
  const location = useLocation();
  const navigate = useNavigate();

  const open = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.has("about");
  }, [location.search]);

  const show = useCallback(() => {
    const params = new URLSearchParams(location.search);
    if (params.has("about")) {
      return;
    }
    params.set("about", "1");
    const search = params.toString();
    navigate(`${location.pathname}${search ? `?${search}` : ""}`, { replace: false });
  }, [location.pathname, location.search, navigate]);

  const hide = useCallback(() => {
    const params = new URLSearchParams(location.search);
    if (!params.has("about")) {
      return;
    }
    params.delete("about");
    const search = params.toString();
    navigate(`${location.pathname}${search ? `?${search}` : ""}`, { replace: true });
  }, [location.pathname, location.search, navigate]);

  return { open, show, hide };
};
