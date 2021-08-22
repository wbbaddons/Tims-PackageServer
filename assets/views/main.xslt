<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:ns="http://www.woltlab.com" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:svg="http://www.w3.org/2000/svg" version="1.0">
	<xsl:output method="html" encoding="UTF-8" doctype-system="about:legacy-compat" />

	<xsl:template match="/">
		<html>
			<head>
				<meta charset="utf-8" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />

				<link rel="stylesheet" href='{{ self.asset("static/bootstrap.min.css")|safe }}' integrity='{{ self.sri("static/bootstrap.min.css")|safe }}' />
				<link rel="stylesheet" href='{{ self.asset("static/main.css")|safe }}' integrity='{{ self.sri("static/main.css")|safe }}' />
				<link rel="icon" href='{{ self.asset("favicon.ico")|safe }}' integrity='{{ self.sri("static/favicon.ico")|safe }}' />

				<title>
					{%- if title.is_some() -%}
						{{- title.as_ref().unwrap() -}}
					{%- else -%}
						{{- fluent!(self.lang, "product-name") -}}
					{%- endif -%}
				</title>
			</head>
			<body>
				<div id="main-grid">
					<aside id="sidebar" class="text-white bg-dark p-3 d-none d-md-grid">
						<div>
							<span class="d-flex flex-column align-items-center mb-3 mb-md-0 me-md-auto fs-4">
								<a class="link-light text-decoration-none" href="{{ host|safe }}">
									<span class="d-flex align-items-center">
										<img src='{{ self.asset("favicon.ico")|safe }}' alt="" width="32" height="32" class="me-2" />
										{%- if title.is_some() -%}
											{{- title.as_ref().unwrap() -}}
										{%- else -%}
											{{- fluent!(self.lang, "product-name") -}}
										{%- endif -%}
									</span>
								</a>
							</span>
							<hr />
						</div>

						<ul id="sidebar-nav" class="nav nav-pills flex-column flex-nowrap overflow-auto">
							<xsl:for-each select="ns:section/ns:package">
								<xsl:sort select="ns:packageinformation/ns:packagename" />

								<li class="package">
									<xsl:attribute name="id">nav-<xsl:value-of select="translate(@name, '.', '-')" /></xsl:attribute>

									<a class="nav-link link-light">
										<xsl:attribute name="href">#<xsl:value-of select="translate(@name, '.', '-')" /></xsl:attribute>

										<xsl:value-of select="./ns:packageinformation/ns:packagename" />
									</a>
								</li>
							</xsl:for-each>
						</ul>
					</aside>

					<main id="main">
						<nav class="navbar navbar-expand-lg navbar-dark bg-dark">
							<div class="container-fluid">
								<a class="navbar-brand d-md-none" href="{{ host|safe }}">
									<img src='{{ self.asset("favicon.ico")|safe }}' alt="" width="24" height="24" class="me-2 d-inline-block align-text-top" />
									{%- if title.is_some() -%}
										{{- title.as_ref().unwrap() -}}
									{%- else -%}
										{{- fluent!(self.lang, "product-name") -}}
									{%- endif -%}
								</a>

								<button class="navbar-toggler ms-auto" type="button" data-bs-toggle="collapse" data-bs-target="#navbarContent" aria-controls="navbarContent" aria-expanded="false" aria-label='{{ fluent!(self.lang, "toggle-navigation") }}'>
									<span class="navbar-toggler-icon"></span>
								</button>

								<div class="collapse navbar-collapse" id="navbarContent">
									{%- if auth_info.username.is_some() -%}
										<span class="navbar-text me-auto mb-2 mb-lg-0">
											{{ fluent!(self.lang, "signed-in-as", { self.auth_info.username }) }}
										</span>
									{%- else -%}
										<a class="btn btn-primary me-auto mb-2 mb-lg-0" href="{{ host|safe }}/login">
											<svg:svg alt="" width="24" height="24" class="bi me-1" fill="currentColor">
												<svg:use href='{{ self.asset("static/icons.svg")|safe }}#box-arrow-in-right' />
											</svg:svg>
											{{ fluent!(self.lang, "sign-in") }}
										</a>
									{%- endif -%}

									<form class="d-flex mb-2 mb-lg-0">
										<input id="search" class="form-control me-2" type="search" maxlength="32" placeholder='{{ fluent!(self.lang, "search-placeholder") }}' aria-label='{{ fluent!(self.lang, "search-placeholder") }}' />
									</form>

									<ul class="navbar-nav">
										<li class="nav-item">
											<a class="nav-link" href='{{ fluent!(self.lang, "github-url") }}'>
												<svg:svg alt="" width="24" height="24" class="bi me-2" fill="currentColor">
													<svg:use href='{{ self.asset("static/icons.svg")|safe }}#github' />
												</svg:svg>
												{{- fluent!(self.lang, "code-on-github") -}}
											</a>
										</li>
									</ul>
								</div>
							</div>
						</nav>

						<div id="main-content" class="container-fluid p-3 overflow-auto" data-bs-spy="scroll" data-bs-target="#sidebar" data-bs-offset="160">
							<div id="no-results" class="card border-warning mb-3 d-none">
								<div class="card-header bg-warning text-dark">{{ fluent!(self.lang, "no-results-heading") }}</div>
								<div class="card-body">
									{{ fluent!(self.lang, "no-results-body") }}
								</div>
							</div>

							<xsl:for-each select="ns:section/ns:package">
								<xsl:sort select="ns:packageinformation/ns:packagename" />

								<div class="anchor-fix package">
									<xsl:attribute name="id"><xsl:value-of select="translate(@name, '.', '-')" /></xsl:attribute>

									<div class="card mb-3">
										<xsl:if test="ns:packageinformation/ns:isapplication='1'">
											<xsl:attribute name="class">card mb-2 border-primary</xsl:attribute>
										</xsl:if>

										<div class="card-header">
											<xsl:if test="ns:packageinformation/ns:isapplication='1'">
												<xsl:attribute name="class">text-white card-header bg-primary</xsl:attribute>

												<svg:svg data-bs-toggle="tooltip" data-bs-container="#main-content" title='{{ fluent!(self.lang, "is-application") }}' alt='{{ fluent!(self.lang, "is-application") }}' width="24" height="24" class="bi me-2" fill="currentColor">
													<svg:use href='{{ self.asset("static/icons.svg")|safe }}#stack' />
												</svg:svg>
											</xsl:if>

											<xsl:value-of select="ns:packageinformation/ns:packagename" /> (<xsl:value-of select="@name" />)

											<xsl:if test="ns:authorinformation">
												<xsl:choose>
													<xsl:when test="ns:authorinformation/ns:authorurl">
														{{ fluent!(self.lang, "by") }}
														<a>
															<xsl:if test="ns:packageinformation/ns:isapplication='1'">
																<xsl:attribute name="class">link-light</xsl:attribute>
															</xsl:if>

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
										</div>

										<div class="card-body px-0 pb-0">
											<xsl:if test="ns:packageinformation/ns:packagedescription!=''">
												<p class="card-text mx-3">
													<xsl:value-of select="ns:packageinformation/ns:packagedescription" />
												</p>
											</xsl:if>

											<div class="table-responsive">
												<table class="table table-striped mb-0">
													<xsl:variable name="has-optionals" select="boolean(ns:versions/ns:version/ns:optionalpackages/ns:optionalpackage)" />
													<thead>
														<tr>
															<th scope="col" class="col-md-2 ps-4">{{ fluent!(self.lang, "version") }}</th>
															<th scope="col" class="col-md-4">{{ fluent!(self.lang, "license") }}</th>
															<th scope="col">
																<xsl:attribute name="class">
																	<xsl:choose>
																		<xsl:when test="$has-optionals">col-md-2</xsl:when>
																		<xsl:otherwise>col-md-4</xsl:otherwise>
																	</xsl:choose>
																</xsl:attribute>
																{{- fluent!(self.lang, "required-packages") -}}
															</th>

															<xsl:if test="$has-optionals">
																<th scope="col" class="col-md-2">{{ fluent!(self.lang, "optional-packages") }}</th>
															</xsl:if>

															<th scope="col" class="col-md-2 pe-4">{{ fluent!(self.lang, "last-modified") }}</th>
														</tr>
													</thead>
													<tbody>
														<xsl:for-each select="ns:versions/ns:version">
															<xsl:sort select="position()" data-type="number" order="descending" />

															<tr>
																<th scope="row" class="ps-4">
																	<!-- Latest version anchor -->
																	<xsl:if test="position()=1">
																		<span class="anchor-fix">
																			<xsl:attribute name="id"><xsl:value-of select="translate(translate(../../@name, ' ', '-'), '.', '-')" />-latest</xsl:attribute>
																		</span>
																	</xsl:if>

																	<!-- Version anchor -->
																	<span class="anchor-fix">
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
																</th>
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

																<xsl:choose>
																	<xsl:when test="ns:optionalpackages/ns:optionalpackage">
																		<td>
																			<ul>
																				<xsl:for-each select="ns:optionalpackages/ns:optionalpackage">
																					<xsl:choose>
																						<xsl:when test="/ns:section/ns:package[@name=current()]">
																							<li>
																								<a>
																									<xsl:attribute name="href">
																										#<xsl:value-of select="translate(., '.', '-')" />
																									</xsl:attribute>

																									<xsl:value-of select="." />
																								</a>
																							</li>
																						</xsl:when>
																						<xsl:otherwise>
																							<li>
																								<xsl:value-of select="." />
																							</li>
																						</xsl:otherwise>
																					</xsl:choose>
																				</xsl:for-each>
																			</ul>
																		</td>
																	</xsl:when>

																	<xsl:when test="$has-optionals">
																		<td></td>
																	</xsl:when>

																	<xsl:otherwise></xsl:otherwise>
																</xsl:choose>

																<td class="pe-4">
																	<time>
																		<xsl:attribute name="data-timestamp">
																			<xsl:value-of select="ns:timestamp" />
																		</xsl:attribute>

																		<xsl:attribute name="datetime">
																			<xsl:call-template name="dateTime">
																				<xsl:with-param name="unixTime" select="ns:timestamp" />
																			</xsl:call-template>
																		</xsl:attribute>

																		<xsl:call-template name="dateTime">
																			<xsl:with-param name="unixTime" select="ns:timestamp" />
																		</xsl:call-template>
																	</time>
																</td>
															</tr>
														</xsl:for-each>
													</tbody>
												</table>
											</div>
										</div>
									</div>
								</div>
							</xsl:for-each>

							<div id="license-info" class="anchor-fix">
								<div class="card">
									<div class="card-body">
										<p>{{ fluent!(self.lang, "presented-by", { self.server_version })|safe }}</p>
										<p>{{ fluent!(self.lang, "license-terms")|safe }}</p>
										<p>{{ fluent!(self.lang, "source-code-url")|safe }}</p>
										<p>{{ fluent!(self.lang, "third-party-info")|safe }}</p>
									</div>
								</div>
							</div>
						</div>
					</main>
				</div>

				<script src='{{ self.asset("static/fuse.min.js")|safe }}' integrity='{{ self.sri("static/fuse.min.js")|safe }}'></script>
				<script src='{{ self.asset("static/bootstrap.bundle.min.js")|safe }}' integrity='{{ self.sri("static/bootstrap.bundle.min.js")|safe }}'></script>

				<script>
					window.TPS_packages = [
						<xsl:for-each select="ns:section/ns:package">
							{
								id: '<xsl:value-of select="@name" />',
								name: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:packageinformation/ns:packagename" /></xsl:call-template>',
								description: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:packageinformation/ns:packagedescription" /></xsl:call-template>',
								author: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:authorinformation/ns:author" /></xsl:call-template>',
								authorURL: '<xsl:call-template name="escapeQuotes"><xsl:with-param name="txt" select="ns:authorinformation/ns:authorurl" /></xsl:call-template>',
								versions: [
									<xsl:for-each select="ns:versions/ns:version">
										<xsl:sort select="position()" data-type="number" order="descending" />

										'<xsl:value-of select="@name" />',
									</xsl:for-each>
								]
							},
						</xsl:for-each>
					];
				</script>

				<script src='{{ self.asset("static/main.js")|safe }}' integrity='{{ self.sri("static/main.js")|safe }}'></script>
			</body>
		</html>
	</xsl:template>

	<xsl:template name="escapeQuotes">
		<!-- http://mac-blog.org.ua/xslt-escape-single-quotes/ -->

		<xsl:param name="txt" />
		<xsl:variable name="backSlashQuote">&#92;&#39;</xsl:variable>
		<xsl:variable name="singleQuote">&#39;</xsl:variable>

		<xsl:choose>
			<xsl:when test="string-length($txt) = 0">
				<!-- early return -->
			</xsl:when>

			<xsl:when test="contains($txt, $singleQuote)">
				<xsl:value-of disable-output-escaping="yes" select="concat(substring-before($txt, $singleQuote), $backSlashQuote)" />

				<xsl:call-template name="escapeQuotes">
					<xsl:with-param name="txt" select="substring-after($txt, $singleQuote)" />
				</xsl:call-template>
			</xsl:when>

			<xsl:otherwise>
				<xsl:value-of disable-output-escaping="yes" select="$txt" />
			</xsl:otherwise>
		</xsl:choose>
	</xsl:template>

	<xsl:template name="dateTime">
		<!-- https://stackoverflow.com/a/58145572 -->

		<xsl:param name="unixTime" />

		<xsl:variable name="JDN" select="floor($unixTime div 86400) + 2440588" />
		<xsl:variable name="secs" select="$unixTime mod 86400" />

		<xsl:variable name="f" select="$JDN + 1401 + floor((floor((4 * $JDN + 274277) div 146097) * 3) div 4) - 38" />
		<xsl:variable name="e" select="4*$f + 3" />
		<xsl:variable name="g" select="floor(($e mod 1461) div 4)" />
		<xsl:variable name="h" select="5*$g + 2" />

		<xsl:variable name="d" select="floor(($h mod 153) div 5 ) + 1" />
		<xsl:variable name="m" select="(floor($h div 153) + 2) mod 12 + 1" />
		<xsl:variable name="y" select="floor($e div 1461) - 4716 + floor((14 - $m) div 12)" />

		<xsl:variable name="H" select="floor($secs div 3600)" />
		<xsl:variable name="M" select="floor($secs mod 3600 div 60)" />
		<xsl:variable name="S" select="$secs mod 60" />

		<xsl:variable name="out">
			<xsl:value-of select="$y" />
			<xsl:text>-</xsl:text>
			<xsl:value-of select="format-number($m, '00')" />
			<xsl:text>-</xsl:text>
			<xsl:value-of select="format-number($d, '00')" />
			<xsl:text>T</xsl:text>
			<xsl:value-of select="format-number($H, '00')" />
			<xsl:text>:</xsl:text>
			<xsl:value-of select="format-number($M, '00')" />
			<xsl:text>:</xsl:text>
			<xsl:value-of select="format-number($S, '00')" />Z
		</xsl:variable>

		<xsl:value-of select="normalize-space($out)" />
	</xsl:template>
</xsl:stylesheet>
