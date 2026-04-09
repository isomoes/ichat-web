<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Register, TryHeaderAuth } from '$lib/api/auth';
	import { _ } from 'svelte-i18n';
	import Button from '$lib/ui/Button.svelte';
	import Input from '$lib/ui/Input.svelte';

	let username = $state('');
	let password = $state('');
	let confirmPassword = $state('');
	let email = $state('');

	let { mutate, isPending, isError } = Register();
	let passwordMismatch = $derived(
		password !== '' && confirmPassword !== '' && password !== confirmPassword
	);
	let disabled = $derived(
		isPending() || username === '' || password === '' || confirmPassword === '' || passwordMismatch
	);

	function completeAuthRedirect() {
		const callback = page.url.searchParams.get('callback');

		if (callback) {
			let url = new URL(decodeURIComponent(callback), document.baseURI);
			if (url.origin == window.location.origin) {
				goto(url);
				return;
			}
		}

		goto('/chat/new');
	}

	function handleSubmit(event: Event) {
		event.preventDefault();
		if (passwordMismatch) return;

		let usernameValue = username;
		let passwordValue = password;
		let emailValue = email === '' ? undefined : email;

		password = '';
		confirmPassword = '';

		mutate(
			{
				username: usernameValue,
				password: passwordValue,
				email: emailValue
			},
			() => {
				completeAuthRedirect();
			}
		);
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
		class="mb-3 bg-gradient-to-r from-secondary to-primary bg-clip-text px-6 text-center text-4xl text-transparent"
	>
		Create your account
	</h2>
	<p class="text-md mb-3 px-6 text-center font-serif">
		Register through ichat. The browser only talks to this server.
	</p>
	<div class="min-w-[80vw] items-center rounded-lg p-6 md:min-w-lg">
		<form class="grid grid-rows-5 gap-4" onsubmit={handleSubmit} inert={isPending()}>
			<div>
				<Input id="username" type="text" placeholder="alice" bind:value={username} required>
					Username
				</Input>
			</div>
			<div>
				<Input id="email" type="email" placeholder="alice@example.com" bind:value={email}>
					Email
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
