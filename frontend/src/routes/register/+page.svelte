<script lang="ts">
	import { goto } from '$app/navigation';
	import { TryHeaderAuth } from '$lib/api/auth';
	import { createMutation } from '$lib/api/state';
	import type { RegisterReq, RegisterResp } from '$lib/api/types';
	import { sendRegisterVerificationEmail } from '$lib/api/register-verification';
	import { _ } from 'svelte-i18n';
	import Button from '$lib/ui/Button.svelte';
	import Input from '$lib/ui/Input.svelte';
	import { canSendVerificationEmail, canSubmitRegistration } from './form';

	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let email = $state('');
	let verificationCode = $state('');
	let isSendingVerification = $state(false);
	let verificationEmailSent = $state(false);

	let { mutate, isPending, isError } = createMutation<RegisterReq, RegisterResp>({
		path: 'auth/register'
	});
	let passwordMismatch = $derived(
		password !== '' && confirmPassword !== '' && password !== confirmPassword
	);
	let disabled = $derived(
		isPending() ||
			isSendingVerification ||
			!canSubmitRegistration({
				username,
				password,
				confirmPassword,
				email,
				verificationCode
			})
	);
	let canSendEmail = $derived(canSendVerificationEmail(email, isSendingVerification, isPending()));

	function completeAuthRedirect() {
		goto('/login');
	}

	function handleSubmit(event: Event) {
		event.preventDefault();
		if (passwordMismatch) return;

		let usernameValue = username.trim();
		let passwordValue = password;
		let emailValue = email.trim();
		let verificationCodeValue = verificationCode.trim();

		password = '';
		confirmPassword = '';

		mutate(
			{
				username: usernameValue,
				password: passwordValue,
				email: emailValue,
				verification_code: verificationCodeValue
			},
			() => {
				completeAuthRedirect();
			}
		);
	}

	async function handleSendVerificationEmail() {
		if (!canSendEmail) return;

		isSendingVerification = true;
		verificationEmailSent = false;

		try {
			const result = await sendRegisterVerificationEmail(email.trim());
			verificationEmailSent = result?.success ?? false;
		} finally {
			isSendingVerification = false;
		}
	}

	$effect(() => {
		TryHeaderAuth();
	});
</script>

<svelte:head>
	<title>{$_('login.title')} - Register</title>
</svelte:head>

<main class="flex h-screen flex-col items-center justify-center bg-login-bg">
	<h2
		class="mb-2 bg-gradient-to-r from-secondary to-primary bg-clip-text px-6 text-center text-4xl text-transparent"
	>
		Create your account
	</h2>
	<p class="text-md mb-2 px-6 text-center font-serif">
		Register through ichat. The browser only talks to this server.
	</p>
	<div class="w-full max-w-md items-center rounded-lg px-6 py-4">
		<form class="grid gap-3" onsubmit={handleSubmit} inert={isPending()}>
			<div>
				<Input id="username" type="text" placeholder="alice" bind:value={username} required>
					Username
				</Input>
			</div>
			<div>
				<Input id="password" type="password" placeholder="P@88w0rd" bind:value={password} required>
					Password
				</Input>
			</div>
			<div>
				<Input
					id="confirm-password"
					type="password"
					placeholder="P@88w0rd"
					bind:value={confirmPassword}
					required
				>
					Confirm password
				</Input>
				{#if passwordMismatch}
					<p class="mt-2 text-sm text-red-400">Passwords do not match.</p>
				{/if}
			</div>
			<div>
				<div class="grid grid-cols-[minmax(0,1fr)_auto] items-end gap-2">
					<div>
						<Input
							id="email"
							type="email"
							placeholder="alice@example.com"
							bind:value={email}
							required
							labelClass="mb-2"
						>
							Email
						</Input>
					</div>
					<Button
						type="button"
						class="mb-0.5 px-3 py-1.5 text-sm whitespace-nowrap"
						disabled={!canSendEmail}
						onclick={handleSendVerificationEmail}
					>
						{#if isSendingVerification}
							Sending code...
						{:else}
							Send code
						{/if}
					</Button>
				</div>
				{#if verificationEmailSent}
					<p class="mt-1 text-sm text-text">Verification email sent.</p>
				{/if}
			</div>
			<div>
				<Input
					id="verification-code"
					type="text"
					placeholder="123456"
					bind:value={verificationCode}
					required
				>
					Verification code
				</Input>
			</div>
			<Button type="submit" class="mt-4 text-lg" {disabled}>
				{#if isError()}
					Try again
				{:else if isPending()}
					Creating account...
				{:else}
					Register
				{/if}
			</Button>
		</form>
		<p class="mt-4 text-center text-sm">
			Already have an account?
			<a href="/login" class="underline">Login</a>
		</p>
	</div>
</main>
