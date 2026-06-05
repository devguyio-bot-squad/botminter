import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

/**
 * EventPicker component tests (CT-04).
 *
 * Tests the event picker dropdown shown during edge creation (drag-to-connect).
 * The picker shows existing trigger events on the destination hat and allows
 * entering a new event name.
 */

import EventPicker from './EventPicker.svelte';

describe('EventPicker component', () => {
	const defaultProps = {
		existingTriggers: ['po.triage', 'lead.plan_review'],
		allTriggers: [
			{ event: 'po.triage', hatId: 'po_gate' },
			{ event: 'lead.plan_review', hatId: 'lead_plan-review' },
			{ event: 'dev.implement', hatId: 'dev_implement-red' }
		],
		onSelect: vi.fn(),
		onCancel: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('shows list of destination hat existing trigger events', () => {
		render(EventPicker, { props: defaultProps });

		// Should display existing triggers as selectable options
		expect(screen.getByText('po.triage')).toBeInTheDocument();
		expect(screen.getByText('lead.plan_review')).toBeInTheDocument();
	});

	it('has text input for new event name', () => {
		render(EventPicker, { props: defaultProps });

		// Should have an input field for entering a new event name
		const input = screen.getByPlaceholderText(/new event/i)
			|| screen.getByRole('textbox');
		expect(input).toBeInTheDocument();
	});

	it('validates new event names — rejects empty string', async () => {
		render(EventPicker, { props: defaultProps });

		const input = screen.getByPlaceholderText(/new event/i)
			|| screen.getByRole('textbox');

		// Submit empty string
		await fireEvent.input(input, { target: { value: '' } });
		const addButton = screen.getByRole('button', { name: /add|create|connect/i });
		await fireEvent.click(addButton);

		// Should show validation error, not call onSelect
		expect(defaultProps.onSelect).not.toHaveBeenCalled();
	});

	it('validates new event names — rejects invalid characters', async () => {
		render(EventPicker, { props: defaultProps });

		const input = screen.getByPlaceholderText(/new event/i)
			|| screen.getByRole('textbox');

		// Enter event name with spaces
		await fireEvent.input(input, { target: { value: 'has spaces' } });
		const addButton = screen.getByRole('button', { name: /add|create|connect/i });
		await fireEvent.click(addButton);

		// Should show validation error
		expect(defaultProps.onSelect).not.toHaveBeenCalled();
		await waitFor(() => {
			const errorEl = screen.queryByText(/invalid/i) || screen.queryByRole('alert');
			expect(errorEl).not.toBeNull();
		});
	});

	it('validates new event names — rejects duplicate trigger with hat name in error', async () => {
		render(EventPicker, { props: defaultProps });

		const input = screen.getByPlaceholderText(/new event/i)
			|| screen.getByRole('textbox');

		// Enter event name that already triggers another hat
		await fireEvent.input(input, { target: { value: 'dev.implement' } });
		const addButton = screen.getByRole('button', { name: /add|create|connect/i });
		await fireEvent.click(addButton);

		// Should show error mentioning the hat that already uses this trigger
		expect(defaultProps.onSelect).not.toHaveBeenCalled();
		await waitFor(() => {
			const errorEl = screen.queryByText(/dev_implement-red/i);
			expect(errorEl).not.toBeNull();
		});
	});

	it('returns selected event name when clicking an existing trigger', async () => {
		render(EventPicker, { props: defaultProps });

		// Click an existing trigger event
		const triggerOption = screen.getByText('po.triage');
		await fireEvent.click(triggerOption);

		expect(defaultProps.onSelect).toHaveBeenCalledWith('po.triage');
	});

	it('returns new event name on valid submission', async () => {
		render(EventPicker, { props: defaultProps });

		const input = screen.getByPlaceholderText(/new event/i)
			|| screen.getByRole('textbox');

		await fireEvent.input(input, { target: { value: 'new.valid.event' } });
		const addButton = screen.getByRole('button', { name: /add|create|connect/i });
		await fireEvent.click(addButton);

		expect(defaultProps.onSelect).toHaveBeenCalledWith('new.valid.event');
	});

	it('returns null on cancel/escape', async () => {
		render(EventPicker, { props: defaultProps });

		// Press Escape to cancel
		await fireEvent.keyDown(document, { key: 'Escape' });

		expect(defaultProps.onCancel).toHaveBeenCalled();
	});

	it('has a cancel button that calls onCancel', async () => {
		render(EventPicker, { props: defaultProps });

		const cancelButton = screen.getByRole('button', { name: /cancel/i });
		await fireEvent.click(cancelButton);

		expect(defaultProps.onCancel).toHaveBeenCalled();
	});
});
