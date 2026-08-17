import type { CapacitorConfig } from "@capacitor/cli";

const config: CapacitorConfig = {
  appId: "fr.divemanager.app",
  appName: "Dive Manager",
  webDir: "dist",
  ios: {
    contentInset: "automatic",
  },
};

export default config;
