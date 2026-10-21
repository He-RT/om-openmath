package org.openmath.openmath.test;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Bitmap;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebView;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.json.JSONObject;
import org.json.JSONTokener;

/** Inspect the signed release APK without enabling debugging in the shipped application. */
public class PackageSmokeInstrumentation extends Instrumentation {
    @Override public void onCreate(android.os.Bundle arguments) {
        super.onCreate(arguments);
        start();
    }
    @Override public void onStart() {
        android.os.Bundle output = new android.os.Bundle();
        try {
            signedPackageComputesTwoRoots();
            output.putString("openmath", "passed");
            finish(Activity.RESULT_OK, output);
        } catch (Throwable error) {
            output.putString("openmath", "failed");
            output.putString("error", android.util.Log.getStackTraceString(error));
            finish(Activity.RESULT_CANCELED, output);
        }
    }
    private void assertTrue(String message, boolean condition) {
        if (!condition) throw new AssertionError(message);
    }
    private void assertNotNull(Object value) { assertNotNull("Expected a value", value); }
    private void assertNotNull(String message, Object value) {
        if (value == null) throw new AssertionError(message);
    }
    private void assertEquals(String message, String expected, String actual) {
        if (!expected.equals(actual)) throw new AssertionError(message);
    }
    private WebView find(View view) {
        if (view instanceof WebView) return (WebView)view;
        if (view instanceof ViewGroup) {
            ViewGroup group=(ViewGroup)view;
            for(int i=0;i<group.getChildCount();i++) { WebView found=find(group.getChildAt(i)); if(found!=null)return found; }
        }
        return null;
    }
    private String evaluate(Activity activity, WebView web, String script) throws Exception {
        CountDownLatch ready=new CountDownLatch(1);
        AtomicReference<String> value=new AtomicReference<>();
        activity.runOnUiThread(()->web.evaluateJavascript(script,result->{value.set(result);ready.countDown();}));
        assertTrue("WebView script callback",ready.await(10,TimeUnit.SECONDS));
        return value.get();
    }
    private void signedPackageComputesTwoRoots() throws Exception {
        Instrumentation instrumentation=this;
        Intent intent=instrumentation.getTargetContext().getPackageManager().getLaunchIntentForPackage("org.openmath.openmath");
        assertNotNull(intent);
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        Activity activity=instrumentation.startActivitySync(intent);
        AtomicReference<WebView> web=new AtomicReference<>();
        long deadline=System.currentTimeMillis()+90000;
        while(web.get()==null && System.currentTimeMillis()<deadline) {
            instrumentation.runOnMainSync(()->web.set(find(activity.getWindow().getDecorView())));
            Thread.sleep(200);
        }
        assertNotNull("Actual Tauri WebView",web.get());
        while(System.currentTimeMillis()<deadline && !"true".equals(evaluate(activity,web.get(),"!!document.querySelector('.add-math')")))Thread.sleep(200);
        String script=new String(instrumentation.getContext().getAssets().open("package-smoke.js").readAllBytes(),StandardCharsets.UTF_8);
        evaluate(activity,web.get(),script);
        JSONObject result=null;
        deadline=System.currentTimeMillis()+100000;
        while(System.currentTimeMillis()<deadline) {
            Object raw=new JSONTokener(evaluate(activity,web.get(),"JSON.stringify(window.__openmathPackageSmoke || {state:'pending'})")).nextValue();
            result=new JSONObject((String)raw);
            if(!result.getString("state").equals("running")&&!result.getString("state").equals("pending"))break;
            Thread.sleep(200);
        }
        assertNotNull(result);
        File folder=instrumentation.getTargetContext().getExternalFilesDir(null);
        try(FileOutputStream out=new FileOutputStream(new File(folder,"package-smoke.json"))) {out.write(result.toString(2).getBytes(StandardCharsets.UTF_8));}
        Bitmap screenshot=instrumentation.getUiAutomation().takeScreenshot();
        if(screenshot!=null)try(FileOutputStream out=new FileOutputStream(new File(folder,"package-smoke.png"))){screenshot.compress(Bitmap.CompressFormat.PNG,100,out);}
        System.out.println("OPENMATH_PACKAGE_SMOKE="+result.toString());
        assertEquals(result.toString(),"passed",result.getString("state"));
    }
}
