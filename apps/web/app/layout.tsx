import type { Metadata } from "next";
import { IBM_Plex_Mono, Space_Grotesk } from "next/font/google";
import Script from "next/script";

import ThemeModeSwitcher from "@/components/theme/theme-mode-switcher";
import { DEFAULT_THEME_MODE, THEME_MODE_STORAGE_KEY } from "@/lib/theme/theme-mode";
import "./globals.css";

const spaceGrotesk = Space_Grotesk({
  variable: "--font-sans",
  subsets: ["latin"],
});

const plexMono = IBM_Plex_Mono({
  variable: "--font-mono",
  subsets: ["latin"],
  weight: ["400", "500", "600"],
});

export const metadata: Metadata = {
  title: "Atlas Hub",
  description:
    "Control plane for Source-in-Git, Distribution-in-Binary modpack releases.",
};

const themeBootScript = `(function(){try{var mode=localStorage.getItem("${THEME_MODE_STORAGE_KEY}")||"${DEFAULT_THEME_MODE}";if(mode!=="light"&&mode!=="dark"&&mode!=="system"){mode="${DEFAULT_THEME_MODE}";}var isDark=mode==="dark"||(mode==="system"&&window.matchMedia("(prefers-color-scheme: dark)").matches);var root=document.documentElement;root.classList.toggle("dark",isDark);root.dataset.themeMode=mode;}catch(e){}})();`;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <Script id="atlas-theme-init" strategy="beforeInteractive">
          {themeBootScript}
        </Script>
      </head>
      <body className={`${spaceGrotesk.variable} ${plexMono.variable} antialiased`}>
        {children}
        <ThemeModeSwitcher className="fixed bottom-4 right-4 z-[100]" />
      </body>
    </html>
  );
}
