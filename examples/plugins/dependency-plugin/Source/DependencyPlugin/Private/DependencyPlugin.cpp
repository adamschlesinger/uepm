#include "DependencyPlugin.h"
#include "ExamplePlugin.h"
#include "Engine/Engine.h"
#include "Math/UnrealMathUtility.h"

#define LOCTEXT_NAMESPACE "FDependencyPluginModule"

void FDependencyPluginModule::StartupModule()
{
    // This code will execute after your module is loaded into memory
    UE_LOG(LogTemp, Warning, TEXT("DependencyPlugin module has been loaded - depends on ExamplePlugin"));
    
    // Verify that ExamplePlugin is available
    if (FExamplePluginModule::IsAvailable())
    {
        UE_LOG(LogTemp, Log, TEXT("DependencyPlugin: ExamplePlugin dependency is available"));
        
        // Demonstrate the integration
        FDependencyPluginFeatures::DemonstratePluginIntegration();
    }
    else
    {
        UE_LOG(LogTemp, Error, TEXT("DependencyPlugin: ExamplePlugin dependency is NOT available!"));
    }
}

void FDependencyPluginModule::ShutdownModule()
{
    UE_LOG(LogTemp, Warning, TEXT("DependencyPlugin module has been unloaded"));
}

// Enhanced feature implementations
FString FDependencyPluginFeatures::CreateColorfulStatusMessage(const FString& Status, int32 Value)
{
    // Use ExamplePlugin utilities to create an enhanced message
    FString FormattedNumber = FExamplePluginUtils::FormatNumberWithCommas(Value);
    FString Greeting = FExamplePluginUtils::GetGreetingMessage(TEXT("System"));
    
    return FString::Printf(TEXT("[%s] Status: %s | Value: %s"), *Greeting, *Status, *FormattedNumber);
}

FVector FDependencyPluginFeatures::GetRandomSpawnLocation(const FVector& Center, float Radius)
{
    // Generate random point in circle
    float Angle = FMath::RandRange(0.0f, 2.0f * PI);
    float Distance = FMath::RandRange(0.0f, Radius);
    
    FVector RandomOffset(
        FMath::Cos(Angle) * Distance,
        FMath::Sin(Angle) * Distance,
        0.0f
    );
    
    FVector SpawnLocation = Center + RandomOffset;
    
    // Use ExamplePlugin to calculate and log the distance
    float DistanceFromCenter = FExamplePluginUtils::CalculateDistance(Center, SpawnLocation);
    UE_LOG(LogTemp, Log, TEXT("Generated spawn location at distance: %s"), 
           *FExamplePluginUtils::FormatNumberWithCommas(FMath::RoundToInt(DistanceFromCenter)));
    
    return SpawnLocation;
}

FString FDependencyPluginFeatures::GenerateSystemReport()
{
    FString Report;
    
    // Header with greeting
    Report += FExamplePluginUtils::GetGreetingMessage(TEXT("System Administrator")) + TEXT("\n");
    Report += TEXT("=== SYSTEM REPORT ===\n");
    
    // Random statistics using ExamplePlugin formatting
    int32 ActiveUsers = FMath::RandRange(1000, 50000);
    int32 TotalConnections = FMath::RandRange(100000, 1000000);
    int32 DataProcessed = FMath::RandRange(1000000, 10000000);
    
    Report += FString::Printf(TEXT("Active Users: %s\n"), 
              *FExamplePluginUtils::FormatNumberWithCommas(ActiveUsers));
    Report += FString::Printf(TEXT("Total Connections: %s\n"), 
              *FExamplePluginUtils::FormatNumberWithCommas(TotalConnections));
    Report += FString::Printf(TEXT("Data Processed: %s bytes\n"), 
              *FExamplePluginUtils::FormatNumberWithCommas(DataProcessed));
    
    // Random color for status
    FLinearColor StatusColor = FExamplePluginUtils::GetRandomColor();
    Report += FString::Printf(TEXT("Status Color: R=%.2f G=%.2f B=%.2f\n"), 
              StatusColor.R, StatusColor.G, StatusColor.B);
    
    Report += TEXT("=== END REPORT ===");
    
    return Report;
}

void FDependencyPluginFeatures::DemonstratePluginIntegration()
{
    UE_LOG(LogTemp, Warning, TEXT("=== DependencyPlugin Integration Demo ==="));
    
    // Test 1: Colorful status message
    FString StatusMsg = CreateColorfulStatusMessage(TEXT("OPERATIONAL"), 12345);
    UE_LOG(LogTemp, Log, TEXT("Status Message: %s"), *StatusMsg);
    
    // Test 2: Random spawn locations
    FVector Center(0, 0, 0);
    for (int32 i = 0; i < 3; i++)
    {
        FVector SpawnLoc = GetRandomSpawnLocation(Center, 1000.0f);
        UE_LOG(LogTemp, Log, TEXT("Spawn Location %d: %s"), i + 1, *SpawnLoc.ToString());
    }
    
    // Test 3: System report
    FString Report = GenerateSystemReport();
    UE_LOG(LogTemp, Log, TEXT("System Report:\n%s"), *Report);
    
    UE_LOG(LogTemp, Warning, TEXT("=== Integration Demo Complete ==="));
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FDependencyPluginModule, DependencyPlugin)