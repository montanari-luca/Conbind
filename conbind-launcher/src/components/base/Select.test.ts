// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Select from './Select.svelte';

const options = [
  { value: 'a', label: 'Alpha' },
  { value: 'b', label: 'Beta' },
];

describe('Select', () => {
  it('shows the current option and reports a change', async () => {
    const onchange = vi.fn();
    render(Select, { props: { value: 'a', options, onchange } });
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.value).toBe('a');
    await fireEvent.change(select, { target: { value: 'b' } });
    expect(onchange).toHaveBeenCalledWith('b');
  });

  it('passes id through so a label can target it', () => {
    render(Select, { props: { id: 'engine', options } });
    expect(screen.getByRole('combobox').id).toBe('engine');
  });

  it('renders a disabled placeholder while nothing is chosen', () => {
    render(Select, { props: { value: '', placeholder: 'Choose', options } });
    const placeholder = screen.getByRole('option', { name: 'Choose' }) as HTMLOptionElement;
    expect(placeholder.disabled).toBe(true);
    expect((screen.getByRole('combobox') as HTMLSelectElement).value).toBe('');
  });

  it('sizes to its content when fill is off', () => {
    const { container } = render(Select, { props: { fill: false, options } });
    expect(container.querySelector('.select-wrap')?.classList.contains('fit')).toBe(true);
  });
});
