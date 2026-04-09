import { beforeEach, describe, expect, it, vi } from 'vitest';

import { sendRegisterVerificationEmail } from './register-verification';

describe('sendRegisterVerificationEmail', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	it('posts the email to the backend auth verification endpoint', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
			new Response(JSON.stringify({ success: true }), {
				status: 200,
				headers: { 'Content-Type': 'application/json' }
			})
		);

		const result = await sendRegisterVerificationEmail('alice@example.com');

		expect(fetchSpy).toHaveBeenCalledWith('/api/auth/verification', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ email: 'alice@example.com' }),
			signal: undefined
		});
		expect(result).toEqual({ success: true });
	});
});
