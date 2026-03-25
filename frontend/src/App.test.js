import { render, screen } from '@testing-library/react';
import App from './App';

test('renders the login form by default', () => {
  render(<App />);
  expect(screen.getByText(/chat app/i)).toBeInTheDocument();
  expect(screen.getByRole('heading', { name: /login/i })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /^login$/i })).toBeInTheDocument();
  expect(screen.getByText(/don't have an account\?/i)).toBeInTheDocument();
});
