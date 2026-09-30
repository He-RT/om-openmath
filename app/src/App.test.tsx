import { render, screen } from '@testing-library/react';
import { expect, test } from 'vitest';
import App from './App';

test('identifies OpenMath without presenting an unfinished solver as usable', () => {
  render(<App />);
  expect(screen.getByRole('heading', { name: 'OpenMath', level: 1 })).toBeTruthy();
  expect(screen.getByText('The solver is under construction.')).toBeTruthy();
  expect(screen.queryByRole('button', { name: /solve|run/i })).toBeNull();
});
