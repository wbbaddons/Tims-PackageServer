const scroll_spy = document.querySelector('[data-bs-spy="scroll"]')

function throttle(fn, threshold = 250, scope) {
	let last = 0
	let deferTimer = null

	return function () {
		const now = new Date().getTime()
		const args = arguments
		const context = scope || this

		if (last && now < last + threshold) {
			clearTimeout(deferTimer)

			return (deferTimer = setTimeout(function () {
				last = now

				return fn.apply(context, args)
			}, threshold))
		} else {
			last = now

			return fn.apply(context, args)
		}
	}
}

function sort_elements(container, selector, order) {
	order = order || default_order

	let elements = []

	order.forEach((e, i, a) => {
		const element = document.querySelector(`${selector}${e}`)
		elements.push(element)
	})

	container.prepend(...elements)
}

function dom_refresh() {
	bootstrap.ScrollSpy.getInstance(scroll_spy).refresh()

	document.documentElement.scrollTop = 0
	document.querySelector('#main-content').dispatchEvent(new Event('scroll'))
}

const date_formatter = new Intl.DateTimeFormat(undefined, {
	year: 'numeric',
	month: 'long',
	day: 'numeric'
})

const datetime_formatter = new Intl.DateTimeFormat(undefined, {
	weekday: 'long',
	hour: '2-digit',
	minute: '2-digit',
	second: '2-digit',
	year: 'numeric',
	month: 'long',
	day: 'numeric'
})

// Format `time` elements
document.querySelectorAll('time').forEach(function(v) {
	const datetime = v.getAttribute('datetime')
	const date = new Date(datetime)

	v.textContent = date_formatter.format(date)

	v.setAttribute('title', datetime_formatter.format(date))
	v.dataset.bsToggle = 'tooltip'
})

// Activate tooltips
const tooltips = [].slice.call(document.querySelectorAll('[data-bs-toggle="tooltip"]'))
tooltips.map(el => new bootstrap.Tooltip(el))

// Initialize search
const options = {
	includeScore: true,
	useExtendedSearch: true,
	keys: [ 'id', 'name', 'description', 'author', 'authorURL', 'versions' ]
}

const fuse = new Fuse(TPS_packages, options)

const default_order = (function() {
	let list = []

	document.querySelectorAll('#main .package').forEach(e => {
		list.push(e.getAttribute('id'))
	})

	return list
})()

const do_search = throttle(function (event) {
	const search_term = event.target.value.substr(0, 32)

	if (search_term.length == 0) {
		requestAnimationFrame(() => {
			sort_elements(document.querySelector('#main-content'), '#')
			sort_elements(document.querySelector('#sidebar-nav'), '#nav-')

			document.querySelectorAll('.package').forEach(e => {
				e.classList.remove('d-none')
			})

			document.querySelector('#no-results').classList.add('d-none')

			dom_refresh()
		})

		return
	}

	const results = fuse
		.search(search_term)
		.map(function (v, k, a) {
			return v.item.id.replace(/\./g, '-')
		})

	requestAnimationFrame(() => {
		if (results.length) {
			document.querySelectorAll('.package').forEach(e => {
				e.classList.add('d-none')
			})

			sort_elements(document.querySelector('#main-content'), '#', results)
			sort_elements(document.querySelector('#sidebar-nav'), '#nav-', results)

			results.forEach(id => {
				document.querySelector(`#${id}`).classList.remove('d-none')
				document.querySelector(`#nav-${id}`).classList.remove('d-none')
			})

			document.querySelector('#no-results').classList.add('d-none')

			dom_refresh()
		}
		else {
			document.querySelectorAll('.package').forEach(e => {
				e.classList.add('d-none')
			})

			document.querySelector('#no-results').classList.remove('d-none')
		}
	})
}, 64)

const search = document.querySelector('#search')
search.addEventListener('input', do_search)
search.addEventListener('change', do_search)

scroll_spy.addEventListener('activate.bs.scrollspy', function (event) {
	const sidebar = document.querySelector('#sidebar-nav')
	const target = document.querySelector('#sidebar-nav .nav-link.active')

	let siblings_height = 0;
	let element = target.parentElement

	while (element = element.previousElementSibling) {
		const style = getComputedStyle(element);

		siblings_height += element.offsetHeight
		siblings_height += parseInt(style.marginTop) + parseInt(style.marginBottom)
	}

	sidebar.scrollTop = siblings_height
})
