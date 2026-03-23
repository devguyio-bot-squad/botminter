/**
 * Deterministic role-to-color mapping.
 *
 * Uses a palette of 20 visually distinct colors. The role name is hashed
 * to an index in this palette, so the same role always gets the same color
 * regardless of profile or team configuration.
 *
 * This is the single source of truth for role colors across the entire dashboard.
 */

const PALETTE = [
	'#1a78d0', // blue
	'#fe6b05', // orange
	'#22c55e', // green
	'#06b6d4', // cyan
	'#a855f7', // purple
	'#ef4444', // red
	'#ec4899', // pink
	'#8b5cf6', // violet
	'#f59e0b', // amber
	'#14b8a6', // teal
	'#3b82f6', // sky blue
	'#f97316', // deep orange
	'#84cc16', // lime
	'#0ea5e9', // light blue
	'#d946ef', // fuchsia
	'#10b981', // emerald
	'#6366f1', // indigo
	'#e11d48', // rose
	'#0891b2', // dark cyan
	'#7c3aed', // dark violet
];

/** Simple string hash (djb2) — deterministic, fast, good distribution. */
function hashString(str: string): number {
	let hash = 5381;
	for (let i = 0; i < str.length; i++) {
		hash = ((hash << 5) + hash + str.charCodeAt(i)) >>> 0;
	}
	return hash;
}

/** Returns a consistent color for any role name. */
export function roleColor(roleName: string): string {
	return PALETTE[hashString(roleName) % PALETTE.length];
}
