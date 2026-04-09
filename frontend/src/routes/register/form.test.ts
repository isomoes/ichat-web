import { describe, expect, it } from 'vitest';

import { canSendVerificationEmail, canSubmitRegistration } from './form';

describe('register form state', () => {
	it('requires email and verification code before submission', () => {
		expect(
			canSubmitRegistration({
				username: 'alice',
				password: 'secret123',
				confirmPassword: 'secret123',
				email: '',
				verificationCode: ''
			})
		).toBe(false);
	});

	it('allows submission when all required fields are present and passwords match', () => {
		expect(
			canSubmitRegistration({
				username: 'alice',
				password: 'secret123',
				confirmPassword: 'secret123',
				email: 'alice@example.com',
				verificationCode: '123456'
			})
		).toBe(true);
	});

	it('only allows sending a verification email when an email is present and no request is pending', () => {
		expect(canSendVerificationEmail('', false, false)).toBe(false);
		expect(canSendVerificationEmail('alice@example.com', true, false)).toBe(false);
		expect(canSendVerificationEmail('alice@example.com', false, true)).toBe(false);
		expect(canSendVerificationEmail('alice@example.com', false, false)).toBe(true);
	});
});
