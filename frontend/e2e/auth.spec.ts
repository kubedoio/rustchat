import { test, expect } from '@playwright/test';

test('should allow a user to sign in', async ({ page }) => {
  // Mock site info
  await page.route('**/api/v1/site/info', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        site_name: 'RustChat',
        logo_url: null
      })
    });
  });

  // Mock login API
  await page.route('**/api/v1/auth/login', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        token: 'fake-jwt-token',
        user: {
          id: '123',
          username: 'testuser',
          email: 'test@example.com',
          role: 'member'
        }
      })
    });
  });

  // Mock me API
  await page.route('**/api/v1/auth/me', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        id: '123',
        username: 'testuser',
        email: 'test@example.com',
        role: 'member'
      })
    });
  });

  // Mock OAuth2 providers
  await page.route('**/api/v1/oauth2/providers', async route => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([])
    });
  });

  await page.goto('/login');

  // Fill in credentials
  await page.fill('#email', 'test@example.com');
  await page.fill('#password', 'password');

  // Click login button
  await page.click('button[type="submit"]');

  // Expect redirect to dashboard (/)
  await expect(page).toHaveURL('/');
});
