import { useEffect, useState } from "react";
import { Box, Card, CardContent, Container, Stack, Typography } from "@mui/material";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

interface EmulatorInfo {
  name?: string;
  version?: string;
  build?: string;
}

export const AboutPane = () => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [emulatorInfo, setEmulatorInfo] = useState<EmulatorInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const devices = useDebuggerDataStore((state) => state.deviceTree);

  useEffect(() => {
    const load = async () => {
      if (!client || connectionState !== "connected") {
        return;
      }
      setError(null);
      try {
        const info = await client.fetchEmulatorInfo();
        if (info) {
          setEmulatorInfo(info);
        } else if (devices.length > 0) {
          setEmulatorInfo({ name: "nullDC", version: "unknown", build: "n/a" });
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    };
    void load();
  }, [client, connectionState, initialized, devices.length]);

  const infoRows = [
    { label: "Name", value: emulatorInfo?.name ?? "—" },
    { label: "Version", value: emulatorInfo?.version ?? "—" },
    { label: "Build", value: emulatorInfo?.build ?? "—" },
  ];

  return (
    <Box
      sx={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        background: (theme) =>
          theme.palette.mode === "dark"
            ? "linear-gradient(135deg, #0f172a 0%, #1e293b 100%)"
            : "linear-gradient(135deg, #f1f5f9 0%, #e0f2fe 100%)",
        py: 12,
      }}
    >
      <Container maxWidth="sm">
        <Card elevation={6} sx={{ borderRadius: 3 }}>
          <CardContent>
            <Stack spacing={3}>
              <Stack spacing={0.5}>
                <Typography variant="h4" fontWeight={700}>
                  About nullDC Debugger
                </Typography>
                <Typography variant="body1" color="text.secondary">
                  Runtime metadata provided by the connected emulator instance.
                </Typography>
              </Stack>

              {error && (
                <Typography variant="body2" color="error">
                  Failed to load emulator info: {error}
                </Typography>
              )}

              <Stack spacing={1.5}>
                {infoRows.map((row) => (
                  <Stack key={row.label} direction="row" justifyContent="space-between">
                    <Typography variant="body2" fontWeight={600}>
                      {row.label}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                      {row.value}
                    </Typography>
                  </Stack>
                ))}
              </Stack>

              <Typography variant="caption" color="text.disabled">
                Connection state: {connectionState}
              </Typography>
            </Stack>
          </CardContent>
        </Card>
      </Container>
    </Box>
  );
};
