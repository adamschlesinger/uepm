#include "ExamplePlugin.h"
#include "Engine/Engine.h"
#include "Math/UnrealMathUtility.h"

#define LOCTEXT_NAMESPACE "FExamplePluginModule"

void FExamplePluginModule::StartupModule()
{
    // This code will execute after your module is loaded into memory; the exact timing is specified in the .uplugin file per-module
    UE_LOG(LogTemp, Warning, TEXT("ExamplePlugin module has been loaded - providing utility functions"));
}

void FExamplePluginModule::ShutdownModule()
{
    // This function may be called during shutdown to clean up your module.  For modules that support dynamic reloading,
    // we call this function before unloading the module.
    UE_LOG(LogTemp, Warning, TEXT("ExamplePlugin module has been unloaded"));
}

// Utility function implementations
FLinearColor FExamplePluginUtils::GetRandomColor()
{
    return FLinearColor(
        FMath::RandRange(0.0f, 1.0f),
        FMath::RandRange(0.0f, 1.0f),
        FMath::RandRange(0.0f, 1.0f),
        1.0f
    );
}

FString FExamplePluginUtils::FormatNumberWithCommas(int32 Number)
{
    FString NumberString = FString::FromInt(Number);
    FString Result;
    
    int32 Count = 0;
    for (int32 i = NumberString.Len() - 1; i >= 0; i--)
    {
        if (Count > 0 && Count % 3 == 0)
        {
            Result = TEXT(",") + Result;
        }
        Result = FString::Chr(NumberString[i]) + Result;
        Count++;
    }
    
    return Result;
}

FString FExamplePluginUtils::GetGreetingMessage(const FString& Name)
{
    FDateTime Now = FDateTime::Now();
    FString TimeString = Now.ToString(TEXT("%H:%M:%S"));
    
    return FString::Printf(TEXT("Hello %s! The current time is %s"), *Name, *TimeString);
}

float FExamplePluginUtils::CalculateDistance(const FVector& A, const FVector& B)
{
    return FVector::Dist(A, B);
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FExamplePluginModule, ExamplePlugin)