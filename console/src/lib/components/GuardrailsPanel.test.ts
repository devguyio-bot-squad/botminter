import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

/**
 * CT-05 Red Phase: GuardrailsPanel unit tests.
 *
 * Tests verify the guardrails panel renders editable textareas,
 * supports adding/removing guardrails, and updates text on input.
 *
 * All tests MUST fail in the red phase -- GuardrailsPanel.svelte
 * does not exist yet.
 */

// Import will fail until GuardrailsPanel.svelte is created
import GuardrailsPanel from './GuardrailsPanel.svelte';

describe('GuardrailsPanel component', () => {
	const sampleGuardrails = [
		'All code must have tests',
		'Follow commit conventions',
		'No secrets in code'
	];

	const defaultProps = {
		guardrails: sampleGuardrails,
		onGuardrailsChange: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe('Rendering guardrails as textareas', () => {
		it('renders each guardrail as a textarea element', () => {
			render(GuardrailsPanel, { props: defaultProps });

			const textareas = screen.getAllByRole('textbox');
			expect(textareas).toHaveLength(3);
		});

		it('textarea values match the guardrail text', () => {
			render(GuardrailsPanel, { props: defaultProps });

			expect(screen.getByDisplayValue('All code must have tests')).toBeInTheDocument();
			expect(screen.getByDisplayValue('Follow commit conventions')).toBeInTheDocument();
			expect(screen.getByDisplayValue('No secrets in code')).toBeInTheDocument();
		});

		it('renders empty list when no guardrails provided', () => {
			render(GuardrailsPanel, {
				props: { guardrails: [], onGuardrailsChange: vi.fn() }
			});

			const textareas = screen.queryAllByRole('textbox');
			expect(textareas).toHaveLength(0);
		});
	});

	describe('Add guardrail button', () => {
		it('renders an Add button', () => {
			render(GuardrailsPanel, { props: defaultProps });

			const addButton = screen.getByRole('button', { name: /add/i });
			expect(addButton).toBeInTheDocument();
		});

		it('clicking Add creates a new empty guardrail entry', async () => {
			render(GuardrailsPanel, { props: defaultProps });

			const addButton = screen.getByRole('button', { name: /add/i });
			await fireEvent.click(addButton);

			expect(defaultProps.onGuardrailsChange).toHaveBeenCalledWith([
				...sampleGuardrails,
				''
			]);
		});
	});

	describe('Remove guardrail button', () => {
		it('each guardrail has a remove (X) button', () => {
			render(GuardrailsPanel, { props: defaultProps });

			// Each guardrail row should have a remove button
			const removeButtons = screen.getAllByRole('button', { name: /remove|delete|x/i });
			expect(removeButtons).toHaveLength(3);
		});

		it('clicking remove button removes the guardrail from the list', async () => {
			render(GuardrailsPanel, { props: defaultProps });

			const removeButtons = screen.getAllByRole('button', { name: /remove|delete|x/i });
			// Remove the second guardrail ("Follow commit conventions")
			await fireEvent.click(removeButtons[1]);

			expect(defaultProps.onGuardrailsChange).toHaveBeenCalledWith([
				'All code must have tests',
				'No secrets in code'
			]);
		});
	});

	describe('Editing guardrail text', () => {
		it('typing in a textarea updates the guardrail text via callback', async () => {
			render(GuardrailsPanel, { props: defaultProps });

			const textareas = screen.getAllByRole('textbox');
			await fireEvent.input(textareas[0], {
				target: { value: 'Updated guardrail text' }
			});

			expect(defaultProps.onGuardrailsChange).toHaveBeenCalledWith([
				'Updated guardrail text',
				'Follow commit conventions',
				'No secrets in code'
			]);
		});
	});
});
