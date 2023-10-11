import "./globals.css";

import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { UserProvider } from "@auth0/nextjs-auth0/client";

const inter = Inter({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: "ParadeDB",
  description: "PostgreSQL for Search",
};

const RootLayout = ({ children }: { children: React.ReactNode }) => (
  <html lang="en">
    <UserProvider>
      <body className={inter.className}>{children}</body>
    </UserProvider>
  </html>
);

export default RootLayout;
