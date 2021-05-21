<script context="module">
	/**
	 * @type {import('@sveltejs/kit').Load}
	 */
	export async function load({ page, fetch, session, context }) {
		return {
			props: {
				path: page.path,
				jwt: session.jwt
			} 
		}
	}

</script>

<script>
	import { goto } from '$app/navigation';

	export let jwt;
	export let path;

	import {
		Dropdown,
		DropdownItem,
		DropdownMenu,
		DropdownToggle,
		Nav,
		NavLink,
		NavItem,
		Modal,
		ModalHeader,
		ModalBody,
		ModalFooter,
		Button
	} from 'sveltestrap';

	import SideBarButton from "$lib/SideBarButton.svelte";

	let signOutModal = false;
	let toggleSignOut = () => signOutModal = !signOutModal

	let signOut = async () => {
		const response = await fetch(`users/logout`, {
          method: 'POST',
          credentials: 'include',
          headers: {
              'Content-Type': 'application/json'
          }
		}).then((r) => r.json());
		goto('/');
	};
</script>
		<div class="d-flex flex-column flex-shrink-0 bg-light" style="width: 4.5rem;">
			<a href="/" class="d-block p-3 link-dark text-decoration-none" title="" data-bs-toggle="tooltip" data-bs-placement="right" data-bs-original-title="Icon-only">
				<svg xmlns="http://www.w3.org/2000/svg" width="40" height="32" fill="currentColor" class="bi bi-app" viewBox="0 0 16 16">
					<path d="M11 2a3 3 0 0 1 3 3v6a3 3 0 0 1-3 3H5a3 3 0 0 1-3-3V5a3 3 0 0 1 3-3h6zM5 1a4 4 0 0 0-4 4v6a4 4 0 0 0 4 4h6a4 4 0 0 0 4-4V5a4 4 0 0 0-4-4H5z"/>
				</svg>
			<span class="visually-hidden">Icon-only</span>
			</a>
			<ul class="nav nav-pills nav-flush flex-column mb-auto text-center">
			<SideBarButton path="/" title="Home" currentPath="{path}">
					<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-house-door" viewBox="0 0 16 16">
											<path d="M8.354 1.146a.5.5 0 0 0-.708 0l-6 6A.5.5 0 0 0 1.5 7.5v7a.5.5 0 0 0 .5.5h4.5a.5.5 0 0 0 .5-.5v-4h2v4a.5.5 0 0 0 .5.5H14a.5.5 0 0 0 .5-.5v-7a.5.5 0 0 0-.146-.354L13 5.793V2.5a.5.5 0 0 0-.5-.5h-1a.5.5 0 0 0-.5.5v1.293L8.354 1.146zM2.5 14V7.707l5.5-5.5 5.5 5.5V14H10v-4a.5.5 0 0 0-.5-.5h-3a.5.5 0 0 0-.5.5v4H2.5z"/>
					</svg>
			</SideBarButton>
			<li>
				<a href="#" class="nav-link py-3 border-bottom" title="" data-bs-toggle="tooltip" data-bs-placement="right" data-bs-original-title="Orders">
				<svg class="bi" width="24" height="24"><use xlink:href="#table"></use></svg>
				</a>
			</li>
			<li>
				<a href="#" class="nav-link py-3 border-bottom" title="" data-bs-toggle="tooltip" data-bs-placement="right" data-bs-original-title="Products">
				<svg class="bi" width="24" height="24"><use xlink:href="#grid"></use></svg>
				</a>
			</li>
			<li>
				<a href="#" class="nav-link py-3 border-bottom" title="" data-bs-toggle="tooltip" data-bs-placement="right" data-bs-original-title="Customers">
				<svg class="bi" width="24" height="24"><use xlink:href="#people-circle"></use></svg>
				</a>
			</li>
			</ul>
			{#if jwt}
				<Dropdown setActiveFromChild class="border-top">
					<DropdownToggle nav class="link-dark d-flex align-items-center justify-content-center p-3 text-decoration-none" caret>
						<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-box-arrow-in-right" viewBox="0 0 16 16">
	0-.708.708L10.293 7.5H4.5z"/>
	<path d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3h3.982a2 2 0 0 1 1.992 2.181l-.637 7A2 2 0 0 1 13.174 14H2.826a2 2 0 0 1-1.991-1.819l-.637-7a1.99 1.99 0 0 1 .342-1.31zM2.19 4a1 1 0 0 0-.996 1.09l.637 7a1 1 0 0 0 .995.91h10.348a1 1 0 0 0 .995-.91l.637-7A1 1 0 0 0 13.81 4H2.19zm4.69-1.707A1 1 0 0 0 6.172 2H2.5a1 1 0 0 0-1 .981l.006.139C1.72 3.042 1.95 3 2.19 3h5.396l-.707-.707z"/>
						</svg>
					</DropdownToggle>
				<DropdownMenu>
					<DropdownItem header>Projects</DropdownItem>
					<DropdownItem href="#" active>Default</DropdownItem>
					<DropdownItem divider />
					<DropdownItem on:click={toggleSignOut}>Sign Out</DropdownItem>
				</DropdownMenu>
			</Dropdown>
			{:else}
			<div class="dropdown border-top">
				<a href="login" class="d-flex align-items-center justify-content-center p-3 text-decoration-none {path === '/login' ? 'bg-primary link-light' : 'link-dark'}" aria-expanded="false">
					<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="currentColor" class="bi bi-box-arrow-in-right" viewBox="0 0 16 16">
						<path fill-rule="evenodd" d="M15 2a1 1 0 0 0-1-1H2a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V2zM0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2V2zm4.5 5.5a.5.5 0 0 0 0 1h5.793l-2.147 2.146a.5.5 0 0 0 .708.708l3-3a.5.5 0 0 0 0-.708l-3-3a.5.5 0 1 0-.708.708L10.293 7.5H4.5z"/>
					</svg>
				</a>
			</div>
			{/if}
		</div>
<div class="container">
	<slot></slot>
</div>

<!-- Sign Out Model -->
<Modal isOpen={signOutModal} {toggleSignOut}>
	<ModalHeader {toggleSignOut}>Sign Out</ModalHeader>
	<ModalBody>
		Are you sure you want to log out?
	</ModalBody>
	<ModalFooter>
		<Button color="primary" on:click={signOut}>Sign Out</Button>
		<Button color="secondary" on:click={toggleSignOut}>Cancel</Button>
	</ModalFooter>
</Modal>
