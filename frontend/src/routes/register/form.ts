export interface RegisterFormState {
	username: string;
	password: string;
	confirmPassword: string;
	email: string;
	verificationCode: string;
}

export function canSubmitRegistration(state: RegisterFormState) {
	return (
		state.username.trim() !== '' &&
		state.password !== '' &&
		state.confirmPassword !== '' &&
		state.email.trim() !== '' &&
		state.verificationCode.trim() !== '' &&
		state.password === state.confirmPassword
	);
}

export function canSendVerificationEmail(
	email: string,
	isSendingVerification: boolean,
	isRegistering: boolean
) {
	return email.trim() !== '' && !isSendingVerification && !isRegistering;
}
