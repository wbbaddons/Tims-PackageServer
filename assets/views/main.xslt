<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:ns="http://www.woltlab.com" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
	<xsl:output method="html" encoding="UTF-8" doctype-system="about:legacy-compat"/>

	<xsl:template match="/">
		<html>
			<head>
				<meta charset="utf-8" />
				<meta http-equiv="X-UA-Compatible" content="IE=edge" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />

				<link rel="icon" href='{{ self.asset("favicon.ico")|safe }}' integrity='{{ self.sri("static/favicon.ico")|safe }}' />
				<link rel="stylesheet" href='{{ self.asset("static/bootstrap.min.css")|safe }}' integrity='{{ self.sri("static/bootstrap.min.css")|safe }}' />

				<style>
					body {
						position: relative;
					}

					#main, .sidebar {
						margin-top: 50px;
					}

					#main {
						padding: 40px 15px;
					}

					.main > .row {
						poisition: relative;
					}

					#sidebarNav {
						padding-top: 40px;
						padding-bottom: 40px;
					}

					.sidebar {
						overflow-y: auto;
						position: fixed;
						top: 0;
						left: 0;
						bottom: 0;
					}

					.jsOnly {
						display: none;
					}

					a.authorURL {
						text-decoration: underline;
					}

					a.authorURL:hover {
						text-decoration: none;
					}

					#noResults {
						display: none;
					}

					/* http://nicolasgallagher.com/jump-links-and-viewport-positioning/demo/#method-C */
					.anchorFix {
						padding-top: 60px;
						margin-top: -60px;
						-webkit-background-clip: content-box;
						background-clip: content-box;
					}
				</style>

				<title>
					{%- if title.is_some() -%}
						{{- title.as_ref().unwrap() -}}
					{%- else -%}
						{{- fluent!(self.lang, "product-name") -}}
					{%- endif -%}
				</title>
			</head>
			<body>
				<div class="navbar navbar-inverse navbar-fixed-top hidden-print" role="navigation">
					<div class="container-fluid">
						<div class="navbar-header">
							<button type="button" class="navbar-toggle collapsed" data-toggle="collapse" data-target=".navbar-collapse">
								<span class="sr-only">Toggle navigation</span>
								<span class="icon-bar"></span>
								<span class="icon-bar"></span>
								<span class="icon-bar"></span>
							</button>
							<a class="navbar-brand" href="#">
								{%- if title.is_some() -%}
									{{- title.as_ref().unwrap() -}}
								{%- else -%}
									{{- fluent!(self.lang, "product-name") -}}
								{%- endif -%}
							</a>
						</div>
						<div class="navbar-collapse collapse">
							{%- if auth_info.username.is_some() -%}
								<p class="navbar-text">
									{{ fluent!(self.lang, "signed-in-as", { self.auth_info.username }) }}
								</p>
							{%- else -%}
								<a href="{{ host|safe }}/login"><button type="button" class="btn btn-default navbar-btn">{{ fluent!(self.lang, "sign-in") }}</button></a>
							{%- endif -%}

							<ul class="nav navbar-nav navbar-right">
								<li><a href="{{ fluent!(self.lang, "github-url") }}">
									{{- fluent!(self.lang, "code-on-github") -}}
								</a></li>
							</ul>

							<div class="navbar-form navbar-right jsOnly">
								<input id="search" type="text" class="form-control" maxlength="32" placeholder="{{ fluent!(self.lang, "search-placeholder") }}" />
							</div>
						</div>
					</div>
				</div>

				<div class="main container-fluid">
					<div class="row">
						<div class="col-md-2 sidebar hidden-xs hidden-sm hidden-print">
							<div id="sidebarNav">
								<ul class="nav nav-pills nav-stacked">
									<xsl:for-each select="ns:section/ns:package">
										<xsl:sort select="ns:packageinformation/ns:packagename" />

										<li class="jsPackage">
											<xsl:attribute name="id">nav-<xsl:value-of select="translate(@name, '.', '-')" /></xsl:attribute>

											<a>
												<xsl:attribute name="href">#<xsl:value-of select="translate(@name, '.', '-')" /></xsl:attribute>

												<xsl:value-of select="./ns:packageinformation/ns:packagename" />
											</a>
										</li>
									</xsl:for-each>
								</ul>
							</div>
						</div>

						<div id="main" class="col-md-10 col-md-offset-2" role="main">
							<xsl:for-each select="ns:section/ns:package">
								<xsl:sort select="ns:packageinformation/ns:packagename" />

								<div class="jsPackage anchorFix">
									<xsl:attribute name="id"><xsl:value-of select="translate(@name, '.', '-')" /></xsl:attribute>

									<div class="panel panel-default packagePanel">
										<xsl:if test="ns:packageinformation/ns:isapplication='1'">
											<xsl:attribute name="class">panel panel-primary packagePanel</xsl:attribute>
										</xsl:if>

										<div class="panel-heading">
											<h3 class="panel-title">
												<xsl:value-of select="ns:packageinformation/ns:packagename" /> (<xsl:value-of select="@name" />)

												<xsl:if test="ns:authorinformation">
													<xsl:choose>
														<xsl:when test="ns:authorinformation/ns:authorurl">
															{{ fluent!(self.lang, "by") }}
															<a class="authorURL">
																<xsl:attribute name="href"><xsl:value-of select="ns:authorinformation/ns:authorurl" /></xsl:attribute>

																<xsl:choose>
																	<xsl:when test="ns:authorinformation/ns:author">
																		<xsl:value-of select="ns:authorinformation/ns:author" />
																	</xsl:when>
																	<xsl:otherwise>
																		<xsl:value-of select="ns:authorinformation/ns:authorurl" />
																	</xsl:otherwise>
																</xsl:choose>
															</a>
														</xsl:when>
														<xsl:otherwise>
															<xsl:if test="ns:authorinformation/ns:author">
																{{ fluent!(self.lang, "by") }} <xsl:value-of select="ns:authorinformation/ns:author" />
															</xsl:if>
														</xsl:otherwise>
													</xsl:choose>
												</xsl:if>
											</h3>
										</div>
										<xsl:if test="ns:packageinformation/ns:packagedescription!=''">
											<div class="panel-body">
												<xsl:value-of select="ns:packageinformation/ns:packagedescription" />
											</div>
										</xsl:if>
										<div class="table-responsive">
											<table class="table table-striped">
												<thead>
													<tr>
														<th class="col-md-2">{{ fluent!(self.lang, "version") }}</th>
														<th class="col-md-4">{{ fluent!(self.lang, "license") }}</th>
														<th class="col-md-4">{{ fluent!(self.lang, "required-packages") }}</th>
														<th class="col-md-2 jsOnly">{{ fluent!(self.lang, "last-modified") }}</th>
													</tr>
												</thead>
												<tbody>
													<xsl:for-each select="ns:versions/ns:version">
														<xsl:sort select="position()" data-type="number" order="descending" />

														<tr>
															<td>
																<!-- Latest version anchor -->
																<xsl:if test="position()=1">
																	<span class="anchorFix">
																		<xsl:attribute name="id"><xsl:value-of select="translate(translate(../../@name, ' ', '-'), '.', '-')" />-latest</xsl:attribute>
																	</span>
																</xsl:if>

																<!-- Version anchor -->
																<span class="anchorFix">
																	<xsl:attribute name="id"><xsl:value-of select="translate(translate(../../@name, ' ', '-'), '.', '-')" />-<xsl:value-of select="translate(translate(@name, ' ', '-'), '.', '-')" /></xsl:attribute>
																</span>

																<xsl:choose>
																	<xsl:when test="@accessible='true'">
																		<a>
																			<xsl:attribute name="href"><xsl:value-of select="ns:file" /></xsl:attribute>

																			<xsl:value-of select="@name" />
																		</a>
																	</xsl:when>
																	<xsl:otherwise>
																		<xsl:value-of select="@name" />
																	</xsl:otherwise>
																</xsl:choose>
															</td>
															<td>
																<xsl:choose>
																	<xsl:when test="ns:license">
																		<xsl:choose>
																			<xsl:when test="ns:license/@url">
																				<a>
																					<xsl:attribute name="href"><xsl:value-of select="ns:license/@url" /></xsl:attribute>
																					<xsl:value-of select="ns:license" />
																				</a>
																			</xsl:when>
																			<xsl:otherwise>
																				<xsl:value-of select="ns:license" />
																			</xsl:otherwise>
																		</xsl:choose>
																	</xsl:when>
																	<xsl:otherwise>{{ fluent!(self.lang, "no-license-information") }}</xsl:otherwise>
																</xsl:choose>
															</td>
															<td>
																<xsl:choose>
																	<xsl:when test="ns:requiredpackages/ns:requiredpackage">
																		<ul>
																			<xsl:for-each select="ns:requiredpackages/ns:requiredpackage">
																				<xsl:choose>
																					<xsl:when test="/ns:section/ns:package[@name=current()]">
																						<li>
																							<a>
																								<xsl:attribute name="href">
																									#<xsl:value-of select="translate(., '.', '-')" />
																								</xsl:attribute>

																								<xsl:value-of select="." /> (<xsl:value-of select="@minversion" />)
																							</a>
																						</li>
																					</xsl:when>
																					<xsl:otherwise>
																						<li>
																							<xsl:value-of select="." /> (<xsl:value-of select="@minversion" />)
																						</li>
																					</xsl:otherwise>
																				</xsl:choose>
																			</xsl:for-each>
																		</ul>
																	</xsl:when>
																	<xsl:otherwise>{{ fluent!(self.lang, "no-requirements") }}</xsl:otherwise>
																</xsl:choose>
															</td>
															<td class="jsOnly">
																<time>
																	<xsl:value-of select="ns:timestamp" />
																</time>
															</td>
														</tr>
													</xsl:for-each>
												</tbody>
											</table>
										</div>
									</div>
								</div>
							</xsl:for-each>

							<div id="noResults" class="panel panel-warning">
								<div class="panel-heading">{{ fluent!(self.lang, "no-results-heading") }}</div>
								<div class="panel-body">
									{{ fluent!(self.lang, "no-results-body") }}
								</div>
							</div>

							<div id="timsPackageServerLicenseInfo" class="anchorFix">
								<div class="panel panel-default">
									<div class="panel-body">
										<p>{{ fluent!(self.lang, "presented-by", { self.server_version })|safe }}</p>
										<p>{{ fluent!(self.lang, "license-terms")|safe }}</p>
										<p>{{ fluent!(self.lang, "source-code-url")|safe }}</p>
										<p>
											{{ fluent!(self.lang, "made-possible-by") }} <button data-toggle="collapse" data-target="#licenseInfo">{{ fluent!(self.lang, "show") }}</button>
											<div id="licenseInfo" class="collapse" style="margin-top: 10px">
												<table class="table table-striped table-bordered table-hover table-condensed">
													<thead>
														<tr>
															<th>{{ fluent!(self.lang, "name") }}</th>
															<th>{{ fluent!(self.lang, "version") }}</th>
															<th>{{ fluent!(self.lang, "license") }}</th>
															<th>{{ fluent!(self.lang, "authors") }}</th>
														</tr>
													</thead>
													<tbody>
														{% for lib in license_info %}
															<tr>
																<td><a href="https://crates.io/crates/{{ lib.0 }}/{{ lib.1 }}">{{ lib.0 }}</a></td>
																<td>{{ lib.1 }}</td>
																<td>{{ lib.2 }}</td>
																<td>
																	{% for author in lib.3 %}
																		{{ author }}<br />
																	{% endfor %}
																</td>
															</tr>
														{% endfor %}
													</tbody>
												</table>
											</div>
										</p>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>

				<script src='{{ self.asset("static/jquery.min.js")|safe }}' integrity='{{ self.sri("static/jquery.min.js")|safe }}'></script>
				<script src='{{ self.asset("static/moment-with-locales.min.js")|safe }}' integrity='{{ self.sri("static/moment-with-locales.min.js")|safe }}'></script>
				<script src='{{ self.asset("static/bootstrap.min.js")|safe }}' integrity='{{ self.sri("static/bootstrap.min.js")|safe }}'></script>
				<script src='{{ self.asset("static/fuse.min.js")|safe }}' integrity='{{ self.sri("static/fuse.min.js")|safe }}'></script>

				<script>
					window.packageSearch = [
						<xsl:for-each select="ns:section/ns:package">
							{
								id: '<xsl:value-of select="@name" />',
								name: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:packageinformation/ns:packagename" /></xsl:call-template>',
								description: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:packageinformation/ns:packagedescription" /></xsl:call-template>',
								author: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:authorinformation/ns:author" /></xsl:call-template>',
								authorURL: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:authorinformation/ns:authorurl" /></xsl:call-template>'
							},
						</xsl:for-each>
					];
				</script>

				<script>
					$(document).ready(function () {
						var fuse = new Fuse(packageSearch, {
							keys: ['name', 'description'],
							includeScore: true,
							threshold: 0.5,
							distance: 100
						});

						window.packageSearch = null; // Free some memory, hopefully

						requestAnimationFrame = (function() {
							if ("requestAnimationFrame" in window) {
								return window.requestAnimationFrame;
							}
							else {
								return function(call) {
									if (typeof call === 'function') {
										call();
									}
								}
							}
						})();

						(function(window) {
							var defaultOrder = (function() {
								var list = [];

								$('#main .jsPackage').each(function() {
									list.push($(this).attr('id'));
								});

								return list;
							})();

							var sortDOMElements = function(container, selector, order) {
								order = order || defaultOrder;

								var elements = [];

								$.each(order, function(index, value) {
									elements.push($(selector + value));
								});

								container.prepend(elements);
							};

							_doSearch = function(value) {
								value = value.substr(0, 32);

								if (!value.length) {
									sortDOMElements($('#main'), '#');
									sortDOMElements($('#sidebarNav > ul'), '#nav-');

									$('.jsPackage').show();
									$('#noResults').hide();

									$('body').scrollspy('refresh');
									$('body').trigger('scroll');

									return;
								}

								var results = fuse.search(value);

								results = results.map(function(v, k, a) {
									return v.item.id.replace(/\./g, '-');
								});

								if (results.length) {
									$('.jsPackage').hide();

									sortDOMElements($('#main'), '#', results);
									sortDOMElements($('#sidebarNav > ul'), '#nav-', results);

									$('#' + results.join(', #')).show();
									$('#nav-' + results.join(', #nav-')).show();

									$('#noResults').hide();
								}
								else {
									$('.jsPackage').hide();
									$('#noResults').show();
								}

								$('body').scrollspy('refresh');
								$('body').trigger('scroll');
							};

							window.doSearch = function(value) {
								requestAnimationFrame(function() {
									_doSearch(value, false);
									$(window).scrollTop(0);
								});
							};
						})(window);

						$('#search').on('input change', function() {
							doSearch($(this).val());
						});

						moment.locale(window.navigator.userLanguage || window.navigator.language);

						$('time').each(function(k, v) {
							$(v).html(moment.unix($(v).text()).format('LL'));
						});

						$('.jsOnly').show();

						$('body').scrollspy({
							target: "#sidebarNav",
							offset: 50
						});

						var sidebar = $('#sidebarNav').parent();

						$('ul.nav li').on('activate.bs.scrollspy', function () {
							siblingsHeight = (function() {
								height = 0;
								$(this).prevAll().each(function(k, v) {
									height += $(v).outerHeight(true);
								});

								return height;
							}).call(this);

							sidebar.scrollTop(siblingsHeight);
						});
					});
				</script>
			</body>
		</html>
	</xsl:template>

	<xsl:template name="escapeQuotes">
		<!-- http://mac-blog.org.ua/xslt-escape-single-quotes/ -->

		<xsl:param name="txt"/>
		<xsl:variable name="backSlashQuote">&#92;&#39;</xsl:variable>
		<xsl:variable name="singleQuote">&#39;</xsl:variable>

		<xsl:choose>
			<xsl:when test="string-length($txt) = 0">
				<!-- early return -->
			</xsl:when>

			<xsl:when test="contains($txt, $singleQuote)">
				<xsl:value-of disable-output-escaping="yes" select="concat(substring-before($txt, $singleQuote), $backSlashQuote)"/>

				<xsl:call-template name="escapeQuotes">
					<xsl:with-param name="txt" select="substring-after($txt, $singleQuote)"/>
				</xsl:call-template>
			</xsl:when>

			<xsl:otherwise>
				<xsl:value-of disable-output-escaping="yes" select="$txt"/>
			</xsl:otherwise>
		</xsl:choose>
	</xsl:template>
</xsl:stylesheet>
