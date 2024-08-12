import type { Config } from "tailwindcss";

export const tailwindConfig: Partial<Config> = {
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: "#006FEE",
          foreground: "#e6f1fe",
          50: "#e6f1fe",
          100: "#cce3fd",
          200: "#99c7fb",
          300: "#66aaf9",
          400: "#338ef7",
          500: "#006FEE",
          600: "#005bc4",
          700: "#004493",
          800: "#002e62",
          900: "#001731",
        },
        danger: {
          DEFAULT: "#f05252",
          foreground: "#fdf2f2",
          50: "#fdf2f2",
          100: "#fde8e8",
          200: "#fbd5d5",
          300: "#f8b4b4",
          400: "#f98080",
          500: "#f05252",
          600: "#e02424",
          700: "#c81e1e",
          800: "#9b1c1c",
          900: "#771d1d",
        },
      },
    },
  },
  darkMode: "class",
  plugins: [],
};
