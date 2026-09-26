import { defineConfig } from "vitepress";

export default defineConfig({
  title: "RustChat Documentation",
  description: "Deployment, operations, user, and development documentation for RustChat.",
  lang: "en-US",
  lastUpdated: true,
  cleanUrls: true,
  srcDir: ".",
  srcExclude: ["archive/**", "plans/**", "DOCUMENTATION_REORGANIZATION_PLAN.md", "REORGANIZATION_SUMMARY.md"],
  ignoreDeadLinks: [/^http:\/\/localhost/, /^http:\/\/127\.0\.0\.1/, /^https:\/\/127\.0\.0\.1/],
  themeConfig: {
    siteTitle: "RustChat Docs",
    search: {
      provider: "local",
    },
    nav: [
      { text: "Home", link: "/" },
      { text: "User", link: "/user/quick-start" },
      { text: "Admin", link: "/admin/installation" },
      { text: "Operations", link: "/operations/runbook" },
      { text: "Development", link: "/development/local-setup" },
      { text: "Architecture", link: "/architecture/overview" },
      { text: "Reference", link: "/reference/compatibility-matrix" },
    ],
    sidebar: {
      "/user/": [
        {
          text: "User Guide",
          items: [
            { text: "Overview", link: "/user/README" },
            { text: "Quick Start", link: "/user/quick-start" },
            { text: "Features", link: "/user/features" },
            { text: "Troubleshooting", link: "/user/troubleshooting" },
            { text: "Connection Status", link: "/user/connection-status" },
          ],
        },
      ],
      "/admin/": [
        {
          text: "Admin Guide",
          items: [
            { text: "Overview", link: "/admin/README" },
            { text: "Installation", link: "/admin/installation" },
            { text: "Configuration", link: "/admin/configuration" },
            { text: "Security", link: "/admin/security" },
            { text: "SSO", link: "/admin/sso" },
            { text: "Email", link: "/admin/email" },
            { text: "Push Notifications", link: "/admin/push-notifications" },
            { text: "Scaling", link: "/admin/scaling" },
            { text: "Backup and Restore", link: "/admin/backup-restore" },
            { text: "Reverse Proxy", link: "/admin/reverse-proxy" },
          ],
        },
      ],
      "/operations/": [
        {
          text: "Operations Guide",
          items: [
            { text: "Overview", link: "/operations/README" },
            { text: "Runbook", link: "/operations/runbook" },
            { text: "Docs Site Deployment", link: "/operations/docs-site-deployment" },
          ],
        },
      ],
      "/development/": [
        {
          text: "Development Guide",
          items: [
            { text: "Overview", link: "/development/README" },
            { text: "Local Setup", link: "/development/local-setup" },
            { text: "Contributing", link: "/development/contributing" },
            { text: "Code Style", link: "/development/code-style" },
            { text: "Testing", link: "/development/testing" },
            { text: "Compatibility", link: "/development/compatibility" },
            { text: "Ownership", link: "/development/ownership" },
            { text: "Agent Model", link: "/development/agent-model" },
            { text: "Operating Model", link: "/development/operating-model" },
            { text: "Releasing", link: "/development/releasing" },
          ],
        },
      ],
      "/architecture/": [
        {
          text: "Architecture",
          items: [
            { text: "Overview", link: "/architecture/overview" },
            { text: "Backend", link: "/architecture/backend" },
            { text: "Frontend", link: "/architecture/frontend" },
            { text: "Realtime", link: "/architecture/realtime" },
            { text: "Data Model", link: "/architecture/data-model" },
            { text: "Calls Deployment", link: "/architecture/calls-deployment" },
            { text: "Integrations", link: "/architecture/integrations" },
          ],
        },
      ],
      "/integrations/": [
        {
          text: "Integrations",
          items: [
            { text: "Overview", link: "/integrations/README" },
            { text: "RustShare", link: "/integrations/rustshare" },
          ],
        },
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "Overview", link: "/reference/README" },
            { text: "Compatibility Matrix", link: "/reference/compatibility-matrix" },
          ],
        },
      ],
    },
    footer: {
      message: "RustChat Documentation",
      copyright: "Copyright © RustChat contributors",
    },
  },
});
