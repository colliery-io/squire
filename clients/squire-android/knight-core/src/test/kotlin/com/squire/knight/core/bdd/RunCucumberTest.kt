package com.squire.knight.core.bdd

import io.cucumber.junit.platform.engine.Constants.GLUE_PROPERTY_NAME
import org.junit.platform.suite.api.ConfigurationParameter
import org.junit.platform.suite.api.IncludeEngines
import org.junit.platform.suite.api.SelectClasspathResource
import org.junit.platform.suite.api.Suite

/**
 * JUnit Platform Suite runner for the Knight Gherkin/Cucumber acceptance suite (SQUIRE-T-0115).
 * Discovers `.feature` files under `src/test/resources/features` and binds them to the step
 * definitions in this package. Runs on the plain JVM (no emulator) via `./gradlew :knight-core:test`.
 */
@Suite
@IncludeEngines("cucumber")
@SelectClasspathResource("features")
@ConfigurationParameter(key = GLUE_PROPERTY_NAME, value = "com.squire.knight.core.bdd")
class RunCucumberTest
