import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

// Mock the marked library used for rendering instructions as HTML
vi.mock('marked', () => ({
	marked: vi.fn().mockImplementation((md: string) => `<p>${md}</p>`)
}));

// Import the component under test — this will fail until the component is created
import HatDetailPanel from './HatDetailPanel.svelte';

describe('HatDetailPanel component', () => {
	const defaultProps = {
		name: 'po_gate',
		description: 'Gates human review decisions',
		triggers: ['po.triage', 'po.backlog'],
		publishes: ['po.gate.approved', 'po.gate.failed'],
		instructions: '# PO Gate\n\nReview incoming items.',
		onNameChange: vi.fn(),
		onDescriptionChange: vi.fn(),
		onEditInstructions: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('Name field', () => {
		it('renders hat name as editable text input with current value', () => {
			render(HatDetailPanel, { props: defaultProps });

			const nameInput = screen.getByDisplayValue('po_gate');
			expect(nameInput).toBeInTheDocument();
			expect(nameInput.tagName).toBe('INPUT');
		});

		it('name input change calls update callback with new name', async () => {
			render(HatDetailPanel, { props: defaultProps });

			const nameInput = screen.getByDisplayValue('po_gate');
			await fireEvent.input(nameInput, { target: { value: 'renamed_hat' } });

			expect(defaultProps.onNameChange).toHaveBeenCalledWith('renamed_hat');
		});
	});

	describe('Description field', () => {
		it('renders hat description as editable text input with current value', () => {
			render(HatDetailPanel, { props: defaultProps });

			const descInput = screen.getByDisplayValue('Gates human review decisions');
			expect(descInput).toBeInTheDocument();
			expect(descInput.tagName).toBe('INPUT');
		});

		it('description input change calls update callback with new description', async () => {
			render(HatDetailPanel, { props: defaultProps });

			const descInput = screen.getByDisplayValue('Gates human review decisions');
			await fireEvent.input(descInput, { target: { value: 'Updated description' } });

			expect(defaultProps.onDescriptionChange).toHaveBeenCalledWith('Updated description');
		});
	});

	describe('Triggers list', () => {
		it('renders triggers as bullet list (read-only)', () => {
			render(HatDetailPanel, { props: defaultProps });

			expect(screen.getByText('po.triage')).toBeInTheDocument();
			expect(screen.getByText('po.backlog')).toBeInTheDocument();
		});
	});

	describe('Publishes list', () => {
		it('renders publishes as bullet list (read-only)', () => {
			render(HatDetailPanel, { props: defaultProps });

			expect(screen.getByText('po.gate.approved')).toBeInTheDocument();
			expect(screen.getByText('po.gate.failed')).toBeInTheDocument();
		});
	});

	describe('Contextual hint caption', () => {
		it('shows hint on triggers/publishes: Connect hats on the canvas to change triggers and publishes', () => {
			render(HatDetailPanel, { props: defaultProps });

			expect(
				screen.getByText(/connect hats on the canvas to change triggers and publishes/i)
			).toBeInTheDocument();
		});
	});

	describe('Instructions display', () => {
		it('renders instructions as rendered markdown (HTML via marked)', () => {
			const { container } = render(HatDetailPanel, { props: defaultProps });

			// The instructions section should contain rendered HTML, not raw markdown
			const instructionsArea = container.querySelector('.instructions-content')
				|| container.querySelector('[data-testid="instructions-content"]');
			expect(instructionsArea).not.toBeNull();
			// Should contain HTML rendered from markdown, not the raw markdown text
			expect(instructionsArea!.innerHTML).toContain('<p>');
		});

		it('shows Edit button next to Instructions heading', () => {
			render(HatDetailPanel, { props: defaultProps });

			const editButton = screen.getByRole('button', { name: /edit/i });
			expect(editButton).toBeInTheDocument();
		});
	});
});
