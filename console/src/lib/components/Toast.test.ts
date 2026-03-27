import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Toast from './Toast.svelte';

describe('Toast Component', () => {
	it('renders message when visible', () => {
		render(Toast, { props: { message: 'Operation succeeded', visible: true, type: 'success' } });
		expect(screen.getByText('Operation succeeded')).toBeInTheDocument();
	});

	it('does not render when not visible', () => {
		render(Toast, { props: { message: 'Hidden message', visible: false } });
		expect(screen.queryByText('Hidden message')).not.toBeInTheDocument();
	});

	it('applies success styling', () => {
		render(Toast, { props: { message: 'Success', visible: true, type: 'success' } });
		const toast = screen.getByRole('status');
		expect(toast.className).toContain('text-emerald-400');
	});

	it('applies error styling', () => {
		render(Toast, { props: { message: 'Error', visible: true, type: 'error' } });
		const toast = screen.getByRole('status');
		expect(toast.className).toContain('text-red-400');
	});

	it('applies info styling by default', () => {
		render(Toast, { props: { message: 'Info', visible: true } });
		const toast = screen.getByRole('status');
		expect(toast.className).toContain('text-blue-400');
	});
});
