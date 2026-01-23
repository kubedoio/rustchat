import { test, expect } from '@playwright/test';

test('should allow a user to sign in', async ({ page }) => {
  await page.goto('/login');

  // Fill in credentials
  await page.fill('#email', 'test@example.com');
  await page.fill('#password', 'password');

  // Click login button
  await page.click('button[type="submit"]');

  // Expect redirect to dashboard (/)
  // This might fail if the user doesn't exist, but it defines the expected behavior.
  await expect(page).toHaveURL('/');
});
