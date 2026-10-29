import type { Palette } from '../services/theme/themeMode';

/**
 * Fixed preview colours for the palette picker. They are deliberately literal:
 * each swatch has to show its own palette regardless of which one is active,
 * which the live theme tokens cannot do.
 */
export const PALETTE_SWATCHES: Record<
  Palette,
  { ground: string; surface: string; accent: string; wash: string }
> = {
  'blue-mint': {
    ground: '#0c1422',
    surface: '#18283e',
    accent: '#5eead4',
    wash: 'linear-gradient(135deg, #5b9cff 0%, #5eead4 100%)',
  },
  classic: {
    ground: '#1e1e20',
    surface: '#323236',
    accent: '#007aff',
    wash: 'linear-gradient(135deg, #007aff 0%, #007aff 100%)',
  },
};
