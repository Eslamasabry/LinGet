/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        oled: {
          bg: '#000000',        // Pure black for OLED
          surface: '#0A0A0A',   // Surface color
          surfaceHover: '#1A1A1A',
          border: '#27272A',    // Subtle border
          text: {
            primary: '#FAFAFA',
            secondary: '#A1A1AA',
            muted: '#71717A',
          },
          accent: {
            bg: '#1E3A5F',
            text: '#60A5FA',
          },
        },
      },
    },
  },
  plugins: [],
}
