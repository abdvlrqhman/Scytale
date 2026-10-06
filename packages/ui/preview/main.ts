import { mount } from 'svelte';
import '../src/styles.css';
import App from './App.svelte';

mount(App, { target: document.getElementById('app')! });
