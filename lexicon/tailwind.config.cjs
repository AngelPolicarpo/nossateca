/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        primary: ["Sora", "Inter", "-apple-system", "system-ui", "Segoe UI", "Helvetica", "Arial", "sans-serif"],
      },
      colors: {
        ui: {
          text: "var(--color-text-primary)",
          muted: "var(--color-text-muted)",
          secondary: "var(--color-text-secondary)",
          primary: "var(--color-primary)",
          primaryActive: "var(--color-primary-active)",
          surface: "var(--color-surface-primary)",
          surfaceAlt: "var(--color-surface-alt)",
          focus: "var(--color-focus)",
        },
      },
    },
  },
  plugins: [],
};
