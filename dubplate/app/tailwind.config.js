/**
 * Tailwind 3, which is the convention leptos-shadcn-ui's components are written
 * against: they emit class names like `bg-primary` and `text-muted-foreground`,
 * and those resolve through the role variables in style/shadcn.css.
 *
 * The palette below is only that bridge. The design system itself is
 * style/tokens.css, reached through `var(--…)` in the app's own markup, and
 * nothing here restates a hex.
 */
import { homedir } from 'node:os';
import { join } from 'node:path';

// Where cargo unpacks the crates it downloaded. The shadcn components' class
// names are in their own sources and nowhere else, so this is the only place
// Tailwind can find them.
const registry = join(process.env.CARGO_HOME || join(homedir(), '.cargo'), 'registry', 'src');

/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['selector', '[data-theme="dark"]'],
  content: [
    './index.html',
    './src/**/*.rs',
    // The components' own class names live in their sources, which are in the
    // cargo registry and not in this tree or the build directory. Without this
    // every class only they use is purged, and they render with no padding, no
    // border and no radius while the ones this application also writes survive
    // and look fine, which is a confusing way to be broken.
    join(registry, '*', 'leptos-shadcn-*', 'src', '**', '*.rs'),
  ],
  theme: {
    extend: {
      colors: {
        border: 'hsl(var(--border))',
        input: 'hsl(var(--input))',
        ring: 'hsl(var(--ring))',
        background: 'hsl(var(--background))',
        foreground: 'hsl(var(--foreground))',
        primary: {
          DEFAULT: 'hsl(var(--primary))',
          foreground: 'hsl(var(--primary-foreground))',
        },
        secondary: {
          DEFAULT: 'hsl(var(--secondary))',
          foreground: 'hsl(var(--secondary-foreground))',
        },
        destructive: {
          DEFAULT: 'hsl(var(--destructive))',
          foreground: 'hsl(var(--destructive-foreground))',
        },
        muted: {
          DEFAULT: 'hsl(var(--muted))',
          foreground: 'hsl(var(--muted-foreground))',
        },
        accent: {
          DEFAULT: 'hsl(var(--accent))',
          foreground: 'hsl(var(--accent-foreground))',
        },
        popover: {
          DEFAULT: 'hsl(var(--popover))',
          foreground: 'hsl(var(--popover-foreground))',
        },
        card: {
          DEFAULT: 'hsl(var(--card))',
          foreground: 'hsl(var(--card-foreground))',
        },
      },
      borderRadius: {
        lg: 'var(--radius)',
        md: 'var(--radius)',
        sm: 'var(--radius)',
      },
      fontFamily: {
        display: ['var(--font-display)'],
        sans: ['var(--font-body)'],
        mono: ['var(--font-mono)'],
      },
    },
  },
};
