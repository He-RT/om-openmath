package org.openmath.openmath;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Intent;
import android.graphics.Bitmap;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebView;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import org.json.JSONObject;
import org.json.JSONTokener;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.*;

/** Inspect the signed release APK without enabling debugging in the shipped application. */
@RunWith(AndroidJUnit4.class)
public class PackageSmokeTest {
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
    @Test public void signedPackageComputesTwoRoots() throws Exception {
        Instrumentation instrumentation=InstrumentationRegistry.getInstrumentation();
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
