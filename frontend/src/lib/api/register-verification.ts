import { dispatchError } from '../error';

interface VerificationReq {
	email: string;
}

interface VerificationResp {
	success: boolean;
}

export async function sendRegisterVerificationEmail(email: string) {
	const response = await fetch('/api/auth/verification', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ email } satisfies VerificationReq)
	});

	const data = (await response.json()) as VerificationResp | { error: string; reason?: string };

	if (typeof data === 'object' && data !== null && 'error' in data) {
		dispatchError(data.error, data.reason);
		return;
	}

	return data;
}
