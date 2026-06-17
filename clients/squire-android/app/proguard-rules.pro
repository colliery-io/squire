# R8 / ProGuard rules for the release build.
#
# Room, okhttp, and zxing-android-embedded ship their own consumer rules, so nothing is needed for
# them here. The one thing R8 can break is kotlinx-serialization's reflective serializer lookup, so
# keep the generated `$$serializer` classes and `serializer()` accessors for our @Serializable types
# (the generated :sdk models and the :pairing Session).
#
# NOTE: the BuildConfig.DEBUG-gated demo-creds bypass is intentionally NOT kept — R8 folds
# BuildConfig.DEBUG to a constant `false` in release and removes the dead branch + demoLogin().

-keepattributes RuntimeVisibleAnnotations,AnnotationDefault,*Annotation*

# kotlinx-serialization json internals.
-keepclassmembers class kotlinx.serialization.json.** { *** Companion; }
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}

# Our @Serializable types (SDK models under com.squire.sdk.*, com.squire.pairing.Session, etc.).
-keep,includedescriptorclasses class com.squire.**$$serializer { *; }
-keepclassmembers class com.squire.** {
    *** Companion;
    kotlinx.serialization.KSerializer serializer(...);
}

# Tink (transitive via androidx.security:security-crypto) references errorprone compile-time
# annotations that aren't on the runtime classpath — safe to ignore (R8-suggested).
-dontwarn com.google.errorprone.annotations.**
